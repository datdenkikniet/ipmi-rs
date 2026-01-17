mod common;

use clap::Parser;
use ipmi_rs::{
    app::{ChannelMediumType, GetChannelInfo},
    connection::{Channel, CompletionErrorCode},
    transport::{
        GetLanConfigParameters, IpAddressSource, LanConfigParameter, LanConfigParameterData,
    },
    IpmiError,
};

#[derive(Parser)]
pub struct Command {
    #[clap(flatten)]
    common: common::CommonOpts,
}

#[derive(Clone, Debug, Default)]
struct LanConfig {
    ip_address: Option<String>,
    subnet_mask: Option<String>,
    gateway: Option<String>,
    mac_address: Option<String>,
    ip_source: Option<String>,
    default_gateway_mac: Option<String>,
    backup_gateway: Option<String>,
    backup_gateway_mac: Option<String>,
}

#[derive(Clone, Debug)]
struct ChannelConfig {
    channel_number: u8,
    lan_config: LanConfig,
}

fn main() -> std::io::Result<()> {
    pretty_env_logger::formatted_builder()
        .parse_filters(&std::env::var("RUST_LOG").unwrap_or("info".to_string()))
        .init();

    let command = Command::parse();
    let mut ipmi = command.common.get_connection()?;

    let mut channels = Vec::new();

    for raw in 0x0..=0xF {
        let channel = match Channel::new(raw) {
            Some(channel) => channel,
            None => continue,
        };

        let info = match ipmi.send_recv(GetChannelInfo::new(channel)) {
            Ok(info) => info,
            Err(
                IpmiError::Failed {
                    completion_code:
                        CompletionErrorCode::InvalidDataFieldInRequest
                        | CompletionErrorCode::ParameterOutOfRange,
                    ..
                }
                | IpmiError::Command {
                    completion_code:
                        Some(
                            CompletionErrorCode::InvalidDataFieldInRequest
                            | CompletionErrorCode::ParameterOutOfRange,
                        ),
                    ..
                },
            ) => continue,
            Err(IpmiError::Failed {
                completion_code: CompletionErrorCode::DestinationUnavailable,
                ..
            })
            | Err(IpmiError::Command {
                completion_code: Some(CompletionErrorCode::DestinationUnavailable),
                ..
            }) => continue,
            Err(err) => {
                log::warn!("Get Channel Info failed for 0x{raw:02X}: {err:?}");
                continue;
            }
        };

        if !matches!(
            info.medium_type,
            ChannelMediumType::Lan802_3 | ChannelMediumType::OtherLan
        ) {
            continue;
        }

        let lan_config = fetch_lan_config(&mut ipmi, channel);
        channels.push(ChannelConfig {
            channel_number: info.channel.value(),
            lan_config,
        });
    }

    println!("{}", render_json(&channels));

    Ok(())
}

fn fetch_lan_config(ipmi: &mut common::IpmiConnectionEnum, channel: Channel) -> LanConfig {
    let mut config = LanConfig::default();

    config.ip_address = get_param_string(ipmi, channel, LanConfigParameter::IpAddress);
    config.subnet_mask = get_param_string(ipmi, channel, LanConfigParameter::SubnetMask);
    config.gateway = get_param_string(ipmi, channel, LanConfigParameter::DefaultGatewayAddress);
    config.mac_address = get_param_string(ipmi, channel, LanConfigParameter::MacAddress);
    config.default_gateway_mac =
        get_param_string(ipmi, channel, LanConfigParameter::DefaultGatewayMacAddress);
    config.backup_gateway =
        get_param_string(ipmi, channel, LanConfigParameter::BackupGatewayAddress);
    config.backup_gateway_mac =
        get_param_string(ipmi, channel, LanConfigParameter::BackupGatewayMacAddress);
    config.ip_source = get_param_ip_source(ipmi, channel);

    config
}

fn get_param_string(
    ipmi: &mut common::IpmiConnectionEnum,
    channel: Channel,
    param: LanConfigParameter,
) -> Option<String> {
    let response = match ipmi.send_recv(GetLanConfigParameters::new(channel, param)) {
        Ok(response) => response,
        Err(IpmiError::Failed {
            completion_code: CompletionErrorCode::CommandSpecific(0x80),
            ..
        })
        | Err(IpmiError::Command {
            completion_code: Some(CompletionErrorCode::CommandSpecific(0x80)),
            ..
        }) => return None,
        Err(err) => {
            log::warn!("Get LAN Config {param:?} failed: {err:?}");
            return None;
        }
    };

    match response.parse(param) {
        Ok(LanConfigParameterData::IpAddress(value)) => Some(value.to_string()),
        Ok(LanConfigParameterData::SubnetMask(value)) => Some(value.to_string()),
        Ok(LanConfigParameterData::DefaultGatewayAddress(value)) => Some(value.to_string()),
        Ok(LanConfigParameterData::MacAddress(value)) => Some(value.to_string()),
        Ok(LanConfigParameterData::DefaultGatewayMacAddress(value)) => Some(value.to_string()),
        Ok(LanConfigParameterData::BackupGatewayAddress(value)) => Some(value.to_string()),
        Ok(LanConfigParameterData::BackupGatewayMacAddress(value)) => Some(value.to_string()),
        _ => None,
    }
}

fn get_param_ip_source(ipmi: &mut common::IpmiConnectionEnum, channel: Channel) -> Option<String> {
    let response = match ipmi.send_recv(GetLanConfigParameters::new(
        channel,
        LanConfigParameter::IpAddressSource,
    )) {
        Ok(response) => response,
        Err(IpmiError::Failed {
            completion_code: CompletionErrorCode::CommandSpecific(0x80),
            ..
        })
        | Err(IpmiError::Command {
            completion_code: Some(CompletionErrorCode::CommandSpecific(0x80)),
            ..
        }) => return None,
        Err(err) => {
            log::warn!("Get LAN Config IpAddressSource failed: {err:?}");
            return None;
        }
    };

    match response.parse(LanConfigParameter::IpAddressSource) {
        Ok(LanConfigParameterData::IpAddressSource(value)) => Some(ip_source_label(value)),
        _ => None,
    }
}

fn ip_source_label(value: IpAddressSource) -> String {
    match value {
        IpAddressSource::Unspecified => "Unspecified".to_string(),
        IpAddressSource::Static => "Static".to_string(),
        IpAddressSource::Dhcp => "DHCP".to_string(),
        IpAddressSource::BiosOrSystemSoftware => "BIOS/System software".to_string(),
        IpAddressSource::Other => "Other".to_string(),
        IpAddressSource::Reserved(v) => format!("Reserved (0x{v:02X})"),
    }
}

fn render_json(channels: &[ChannelConfig]) -> String {
    let mut out = String::new();
    out.push_str("{\n  \"channels\": [\n");

    for (index, channel) in channels.iter().enumerate() {
        if index > 0 {
            out.push_str(",\n");
        }

        out.push_str("    {\n");
        out.push_str(&format!(
            "      \"channel_number\": {},\n",
            channel.channel_number
        ));
        out.push_str("      \"lan_config\": {\n");

        let fields = [
            ("ip_address", &channel.lan_config.ip_address),
            ("subnet_mask", &channel.lan_config.subnet_mask),
            ("gateway", &channel.lan_config.gateway),
            ("mac_address", &channel.lan_config.mac_address),
            ("ip_source", &channel.lan_config.ip_source),
            (
                "default_gateway_mac",
                &channel.lan_config.default_gateway_mac,
            ),
            ("backup_gateway", &channel.lan_config.backup_gateway),
            ("backup_gateway_mac", &channel.lan_config.backup_gateway_mac),
        ];

        for (field_index, (name, value)) in fields.iter().enumerate() {
            if field_index > 0 {
                out.push_str(",\n");
            }
            out.push_str(&format!(
                "        \"{}\": {}",
                name,
                json_value(value.as_deref())
            ));
        }

        out.push_str("\n      }\n    }");
    }

    out.push_str("\n  ]\n}\n");
    out
}

fn json_value(value: Option<&str>) -> String {
    match value {
        Some(value) => format!("\"{}\"", escape_json(value)),
        None => "null".to_string(),
    }
}

fn escape_json(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            '\\' => "\\\\".to_string(),
            '"' => "\\\"".to_string(),
            '\n' => "\\n".to_string(),
            '\r' => "\\r".to_string(),
            '\t' => "\\t".to_string(),
            _ => ch.to_string(),
        })
        .collect::<Vec<_>>()
        .join("")
}
