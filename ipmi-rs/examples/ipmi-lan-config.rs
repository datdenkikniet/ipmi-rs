mod common;

use clap::Parser;
use ipmi_rs::{
    app::{ChannelMediumType, GetChannelInfo},
    connection::{Channel, CompletionErrorCode},
    transport::{
        GetLanConfigParameters, IpAddressSource, LanConfigParameter, LanConfigParameterData,
        LanConfigParameterRequest, SetLanConfigParameters,
    },
    IpmiError,
};
use serde::Deserialize;

#[derive(Parser)]
pub struct Command {
    #[clap(flatten)]
    common: common::CommonOpts,
    /// Apply LAN configuration from a JSON file
    #[clap(long)]
    set: Option<String>,
    /// Attempt to write all fields, including ones that are often read-only
    #[clap(long)]
    force_write_all: bool,
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

#[derive(Clone, Debug, Default, Deserialize)]
struct LanConfigInput {
    ip_address: Option<String>,
    subnet_mask: Option<String>,
    gateway: Option<String>,
    mac_address: Option<String>,
    ip_source: Option<String>,
    default_gateway_mac: Option<String>,
    backup_gateway: Option<String>,
    backup_gateway_mac: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct ConfigInput {
    channels: Vec<ChannelInput>,
}

#[derive(Clone, Debug, Deserialize)]
struct ChannelInput {
    channel_number: u8,
    lan_config: LanConfigInput,
}

#[derive(Clone, Debug)]
struct ChannelConfig {
    channel_number: u8,
    lan_config: LanConfig,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum FetchError {
    Timeout,
    OutOfSync,
}

#[derive(Clone, Debug)]
struct FetchResult {
    config: LanConfig,
    aborted: Option<FetchError>,
}

fn main() -> std::io::Result<()> {
    pretty_env_logger::formatted_builder()
        .parse_filters(&std::env::var("RUST_LOG").unwrap_or("info".to_string()))
        .init();

    let command = Command::parse();
    let mut ipmi = command.common.get_connection()?;

    if let Some(path) = command.set.as_deref() {
        apply_config_file(&mut ipmi, path, command.force_write_all)?;
    }

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
            Err(IpmiError::Connection(err)) => {
                if let Some(fetch) = classify_fetch_error(&err) {
                    log_readback_abort(fetch);
                    break;
                }
                log::warn!("Get Channel Info failed for 0x{raw:02X}: {err:?}");
                continue;
            }
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

        let fetch = fetch_lan_config(&mut ipmi, channel);
        channels.push(ChannelConfig {
            channel_number: info.channel.value(),
            lan_config: fetch.config,
        });
        if let Some(err) = fetch.aborted {
            log_readback_abort(err);
            break;
        }
    }

    println!("{}", render_json(&channels));

    Ok(())
}

fn apply_config_file(
    ipmi: &mut common::IpmiConnectionEnum,
    path: &str,
    force_write_all: bool,
) -> std::io::Result<()> {
    let contents = std::fs::read_to_string(path)?;
    let config: ConfigInput = serde_json::from_str(&contents)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;

    for entry in config.channels {
        let channel = match Channel::new(entry.channel_number) {
            Some(channel) => channel,
            None => {
                log::warn!(
                    "Skipping invalid channel number 0x{:02X}",
                    entry.channel_number
                );
                continue;
            }
        };

        apply_lan_config(ipmi, channel, &entry.lan_config, force_write_all);
        wait_for_set_complete(ipmi, channel);
    }

    Ok(())
}

fn apply_lan_config(
    ipmi: &mut common::IpmiConnectionEnum,
    channel: Channel,
    config: &LanConfigInput,
    force_write_all: bool,
) {
    let _ = set_param(
        ipmi,
        channel,
        LanConfigParameter::SetInProgress,
        LanConfigParameterRequest::SetInProgress(0x01),
    );

    if let Some(ip_address) = config.ip_address.as_deref() {
        if let Some(value) = parse_ipv4(ip_address) {
            let _ = set_param(
                ipmi,
                channel,
                LanConfigParameter::IpAddress,
                LanConfigParameterRequest::IpAddress(value),
            );
        } else {
            log::warn!("Invalid IPv4 address: {ip_address}");
        }
    }

    if let Some(subnet_mask) = config.subnet_mask.as_deref() {
        if let Some(value) = parse_ipv4(subnet_mask) {
            let _ = set_param(
                ipmi,
                channel,
                LanConfigParameter::SubnetMask,
                LanConfigParameterRequest::SubnetMask(value),
            );
        } else {
            log::warn!("Invalid subnet mask: {subnet_mask}");
        }
    }

    if let Some(gateway) = config.gateway.as_deref() {
        if let Some(value) = parse_ipv4(gateway) {
            let _ = set_param(
                ipmi,
                channel,
                LanConfigParameter::DefaultGatewayAddress,
                LanConfigParameterRequest::DefaultGatewayAddress(value),
            );
        } else {
            log::warn!("Invalid gateway address: {gateway}");
        }
    }

    if let Some(mac_address) = config.mac_address.as_deref() {
        if !force_write_all {
            log::warn!(
                "Skipping MAC address write; this parameter is often read-only (use --force-write-all to override)"
            );
        } else if let Some(value) = parse_mac(mac_address) {
            let _ = set_param(
                ipmi,
                channel,
                LanConfigParameter::MacAddress,
                LanConfigParameterRequest::MacAddress(value),
            );
        } else {
            log::warn!("Invalid MAC address: {mac_address}");
        }
    }

    if let Some(ip_source) = config.ip_source.as_deref() {
        if let Some(value) = parse_ip_source(ip_source) {
            let _ = set_param(
                ipmi,
                channel,
                LanConfigParameter::IpAddressSource,
                LanConfigParameterRequest::IpAddressSource(value.into()),
            );
        } else {
            log::warn!("Invalid IP source: {ip_source}");
        }
    }

    if let Some(default_gateway_mac) = config.default_gateway_mac.as_deref() {
        if !force_write_all {
            log::warn!(
                "Skipping default gateway MAC write; this parameter is often read-only (use --force-write-all to override)"
            );
        } else if let Some(value) = parse_mac(default_gateway_mac) {
            let _ = set_param(
                ipmi,
                channel,
                LanConfigParameter::DefaultGatewayMacAddress,
                LanConfigParameterRequest::DefaultGatewayMacAddress(value),
            );
        } else {
            log::warn!("Invalid default gateway MAC: {default_gateway_mac}");
        }
    }

    if let Some(backup_gateway) = config.backup_gateway.as_deref() {
        if let Some(value) = parse_ipv4(backup_gateway) {
            let _ = set_param(
                ipmi,
                channel,
                LanConfigParameter::BackupGatewayAddress,
                LanConfigParameterRequest::BackupGatewayAddress(value),
            );
        } else {
            log::warn!("Invalid backup gateway address: {backup_gateway}");
        }
    }

    if let Some(backup_gateway_mac) = config.backup_gateway_mac.as_deref() {
        if !force_write_all {
            log::warn!(
                "Skipping backup gateway MAC write; this parameter is often read-only (use --force-write-all to override)"
            );
        } else if let Some(value) = parse_mac(backup_gateway_mac) {
            let _ = set_param(
                ipmi,
                channel,
                LanConfigParameter::BackupGatewayMacAddress,
                LanConfigParameterRequest::BackupGatewayMacAddress(value),
            );
        } else {
            log::warn!("Invalid backup gateway MAC: {backup_gateway_mac}");
        }
    }

    let _ = set_param(
        ipmi,
        channel,
        LanConfigParameter::SetInProgress,
        LanConfigParameterRequest::SetInProgress(0x00),
    );
}

fn wait_for_set_complete(ipmi: &mut common::IpmiConnectionEnum, channel: Channel) {
    let retry_delay = std::time::Duration::from_millis(250);
    let max_attempts = 20;

    for _ in 0..max_attempts {
        let response = match ipmi.send_recv(GetLanConfigParameters::new(
            channel,
            LanConfigParameter::SetInProgress,
        )) {
            Ok(response) => response,
            Err(IpmiError::Failed {
                completion_code: CompletionErrorCode::CommandSpecific(0x80),
                ..
            })
            | Err(IpmiError::Command {
                completion_code: Some(CompletionErrorCode::CommandSpecific(0x80)),
                ..
            }) => return,
            Err(err) => {
                log::warn!("Get LAN Config SetInProgress failed: {err:?}");
                return;
            }
        };

        if let Some(state) = response.data.get(0) {
            if (state & 0x03) == 0x00 {
                return;
            }
        }

        std::thread::sleep(retry_delay);
    }

    log::warn!("Set In Progress did not complete after polling");
}

fn set_param(
    ipmi: &mut common::IpmiConnectionEnum,
    channel: Channel,
    param: LanConfigParameter,
    request: LanConfigParameterRequest,
) -> bool {
    let response = ipmi.send_recv(SetLanConfigParameters::from_request(
        channel, param, request,
    ));

    match response {
        Ok(_) => true,
        Err(IpmiError::Failed {
            completion_code: CompletionErrorCode::CommandSpecific(0x80),
            ..
        })
        | Err(IpmiError::Command {
            completion_code: Some(CompletionErrorCode::CommandSpecific(0x80)),
            ..
        }) => {
            log::warn!("LAN config parameter {param:?} not supported");
            false
        }
        Err(err) => {
            log::warn!("Set LAN config {param:?} failed: {err:?}");
            false
        }
    }
}

fn parse_ipv4(value: &str) -> Option<ipmi_rs::transport::Ipv4Address> {
    let mut parts = [0u8; 4];
    let mut index = 0;
    for part in value.split('.') {
        if index >= 4 {
            return None;
        }
        let parsed = part.parse::<u8>().ok()?;
        parts[index] = parsed;
        index += 1;
    }
    if index != 4 {
        return None;
    }
    Some(ipmi_rs::transport::Ipv4Address(parts))
}

fn parse_mac(value: &str) -> Option<ipmi_rs::transport::MacAddress> {
    let mut parts = [0u8; 6];
    let mut index = 0;
    for part in value.split(':') {
        if index >= 6 {
            return None;
        }
        let parsed = u8::from_str_radix(part, 16).ok()?;
        parts[index] = parsed;
        index += 1;
    }
    if index != 6 {
        return None;
    }
    Some(ipmi_rs::transport::MacAddress(parts))
}

fn parse_ip_source(value: &str) -> Option<IpAddressSource> {
    let lower = value.to_ascii_lowercase();
    match lower.as_str() {
        "unspecified" => Some(IpAddressSource::Unspecified),
        "static" => Some(IpAddressSource::Static),
        "dhcp" => Some(IpAddressSource::Dhcp),
        "bios" | "bios/system software" | "bios-system software" => {
            Some(IpAddressSource::BiosOrSystemSoftware)
        }
        "other" => Some(IpAddressSource::Other),
        _ => {
            if let Some(hex) = lower.strip_prefix("0x") {
                u8::from_str_radix(hex, 16)
                    .ok()
                    .map(IpAddressSource::Reserved)
            } else {
                None
            }
        }
    }
}

fn fetch_lan_config(ipmi: &mut common::IpmiConnectionEnum, channel: Channel) -> FetchResult {
    let mut config = LanConfig::default();

    if let Err(err) = fill_param(
        &mut config.ip_address,
        ipmi,
        channel,
        LanConfigParameter::IpAddress,
    ) {
        return FetchResult {
            config,
            aborted: Some(err),
        };
    }
    if let Err(err) = fill_param(
        &mut config.subnet_mask,
        ipmi,
        channel,
        LanConfigParameter::SubnetMask,
    ) {
        return FetchResult {
            config,
            aborted: Some(err),
        };
    }
    if let Err(err) = fill_param(
        &mut config.gateway,
        ipmi,
        channel,
        LanConfigParameter::DefaultGatewayAddress,
    ) {
        return FetchResult {
            config,
            aborted: Some(err),
        };
    }
    if let Err(err) = fill_param(
        &mut config.mac_address,
        ipmi,
        channel,
        LanConfigParameter::MacAddress,
    ) {
        return FetchResult {
            config,
            aborted: Some(err),
        };
    }
    if let Err(err) = fill_param(
        &mut config.default_gateway_mac,
        ipmi,
        channel,
        LanConfigParameter::DefaultGatewayMacAddress,
    ) {
        return FetchResult {
            config,
            aborted: Some(err),
        };
    }
    if let Err(err) = fill_param(
        &mut config.backup_gateway,
        ipmi,
        channel,
        LanConfigParameter::BackupGatewayAddress,
    ) {
        return FetchResult {
            config,
            aborted: Some(err),
        };
    }
    if let Err(err) = fill_param(
        &mut config.backup_gateway_mac,
        ipmi,
        channel,
        LanConfigParameter::BackupGatewayMacAddress,
    ) {
        return FetchResult {
            config,
            aborted: Some(err),
        };
    }
    if let Err(err) = fill_ip_source(&mut config.ip_source, ipmi, channel) {
        return FetchResult {
            config,
            aborted: Some(err),
        };
    }

    FetchResult {
        config,
        aborted: None,
    }
}

fn fill_param(
    slot: &mut Option<String>,
    ipmi: &mut common::IpmiConnectionEnum,
    channel: Channel,
    param: LanConfigParameter,
) -> Result<(), FetchError> {
    *slot = get_param_string(ipmi, channel, param)?;
    Ok(())
}

fn fill_ip_source(
    slot: &mut Option<String>,
    ipmi: &mut common::IpmiConnectionEnum,
    channel: Channel,
) -> Result<(), FetchError> {
    *slot = get_param_ip_source(ipmi, channel)?;
    Ok(())
}

fn log_readback_abort(err: FetchError) {
    match err {
        FetchError::Timeout => {
            log::warn!("LAN config readback aborted after timeout; re-run to refresh");
        }
        FetchError::OutOfSync => {
            log::warn!("LAN config readback aborted due to sequence mismatch; re-run to refresh");
        }
    }
}

fn get_param_string(
    ipmi: &mut common::IpmiConnectionEnum,
    channel: Channel,
    param: LanConfigParameter,
) -> Result<Option<String>, FetchError> {
    let response = match retry_get_param(ipmi, channel, param)? {
        Some(response) => response,
        None => return Ok(None),
    };

    match response.parse(param) {
        Ok(LanConfigParameterData::IpAddress(value)) => Ok(Some(value.to_string())),
        Ok(LanConfigParameterData::SubnetMask(value)) => Ok(Some(value.to_string())),
        Ok(LanConfigParameterData::DefaultGatewayAddress(value)) => Ok(Some(value.to_string())),
        Ok(LanConfigParameterData::MacAddress(value)) => Ok(Some(value.to_string())),
        Ok(LanConfigParameterData::DefaultGatewayMacAddress(value)) => Ok(Some(value.to_string())),
        Ok(LanConfigParameterData::BackupGatewayAddress(value)) => Ok(Some(value.to_string())),
        Ok(LanConfigParameterData::BackupGatewayMacAddress(value)) => Ok(Some(value.to_string())),
        _ => Ok(None),
    }
}

fn get_param_ip_source(
    ipmi: &mut common::IpmiConnectionEnum,
    channel: Channel,
) -> Result<Option<String>, FetchError> {
    let response = match retry_get_param(ipmi, channel, LanConfigParameter::IpAddressSource)? {
        Some(response) => response,
        None => return Ok(None),
    };

    match response.parse(LanConfigParameter::IpAddressSource) {
        Ok(LanConfigParameterData::IpAddressSource(value)) => Ok(Some(ip_source_label(value))),
        _ => Ok(None),
    }
}

fn retry_get_param(
    ipmi: &mut common::IpmiConnectionEnum,
    channel: Channel,
    param: LanConfigParameter,
) -> Result<Option<ipmi_rs::transport::LanConfigParameterResponse>, FetchError> {
    let retry_delay = std::time::Duration::from_millis(200);
    let max_attempts = 3;

    for attempt in 0..max_attempts {
        let response = ipmi.send_recv(GetLanConfigParameters::new(channel, param));
        match response {
            Ok(response) => return Ok(Some(response)),
            Err(IpmiError::Failed {
                completion_code: CompletionErrorCode::CommandSpecific(0x80),
                ..
            })
            | Err(IpmiError::Command {
                completion_code: Some(CompletionErrorCode::CommandSpecific(0x80)),
                ..
            }) => return Ok(None),
            Err(IpmiError::Connection(err)) => {
                if let Some(fetch) = classify_fetch_error(&err) {
                    if fetch == FetchError::Timeout && attempt + 1 < max_attempts {
                        std::thread::sleep(retry_delay);
                        continue;
                    }
                    return Err(fetch);
                }
                log::warn!("Get LAN Config {param:?} failed: {err:?}");
                return Err(FetchError::Timeout);
            }
            Err(err) => {
                log::warn!("Get LAN Config {param:?} failed: {err:?}");
                return Err(FetchError::Timeout);
            }
        }
    }

    Err(FetchError::Timeout)
}

fn classify_fetch_error(err: &std::io::Error) -> Option<FetchError> {
    if err.kind() == std::io::ErrorKind::WouldBlock {
        return Some(FetchError::Timeout);
    }

    if err.kind() == std::io::ErrorKind::Other
        && err.to_string().contains("Invalid sequence number")
    {
        return Some(FetchError::OutOfSync);
    }

    None
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
