mod common;

use clap::Parser;
use ipmi_rs::{
    app::{ChannelInfo, GetChannelInfo},
    connection::{Channel, CompletionErrorCode},
    transport::{GetLanConfigParameters, LanConfigParameter, LanConfigParameterData},
    IpmiError,
};

#[derive(Parser)]
pub struct Command {
    #[clap(flatten)]
    common: common::CommonOpts,
}

fn print_channel_info(info: &ChannelInfo) {
    println!("{}", info.channel);
    println!("  Medium: {}", info.medium_type);
    println!("  Protocol: {}", info.protocol_type);
    println!("  Session support: {}", info.session_support);
    println!("  Active sessions: {}", info.active_sessions);
    println!("  Vendor ID: 0x{:06X}", info.vendor_id);
    println!(
        "  Aux: 0x{:02X} 0x{:02X}",
        info.aux_info.byte1, info.aux_info.byte2
    );
}

fn print_lan_config(ipmi: &mut common::IpmiConnectionEnum, channel: Channel) {
    println!("  LAN config:");

    let params = [
        ("IP Address", LanConfigParameter::IpAddress),
        ("IP Source", LanConfigParameter::IpAddressSource),
        ("MAC Address", LanConfigParameter::MacAddress),
        ("Subnet Mask", LanConfigParameter::SubnetMask),
        ("Default Gateway", LanConfigParameter::DefaultGatewayAddress),
        (
            "Default Gateway MAC",
            LanConfigParameter::DefaultGatewayMacAddress,
        ),
        ("Backup Gateway", LanConfigParameter::BackupGatewayAddress),
        (
            "Backup Gateway MAC",
            LanConfigParameter::BackupGatewayMacAddress,
        ),
    ];

    for (label, param) in params {
        let response = ipmi.send_recv(GetLanConfigParameters::new(channel, param));
        let response = match response {
            Ok(response) => response,
            Err(IpmiError::Failed {
                completion_code: CompletionErrorCode::CommandSpecific(0x80),
                ..
            })
            | Err(IpmiError::Command {
                completion_code: Some(CompletionErrorCode::CommandSpecific(0x80)),
                ..
            }) => {
                println!("    {label}: not supported");
                continue;
            }
            Err(err) => {
                println!("    {label}: error ({err:?})");
                continue;
            }
        };

        let parsed = response.parse(param);
        match parsed {
            Ok(LanConfigParameterData::IpAddress(value)) => println!("    {label}: {value}"),
            Ok(LanConfigParameterData::IpAddressSource(value)) => {
                println!("    {label}: {value}")
            }
            Ok(LanConfigParameterData::MacAddress(value)) => println!("    {label}: {value}"),
            Ok(LanConfigParameterData::SubnetMask(value)) => println!("    {label}: {value}"),
            Ok(LanConfigParameterData::DefaultGatewayAddress(value)) => {
                println!("    {label}: {value}")
            }
            Ok(LanConfigParameterData::DefaultGatewayMacAddress(value)) => {
                println!("    {label}: {value}")
            }
            Ok(LanConfigParameterData::BackupGatewayAddress(value)) => {
                println!("    {label}: {value}")
            }
            Ok(LanConfigParameterData::BackupGatewayMacAddress(value)) => {
                println!("    {label}: {value}")
            }
            Ok(LanConfigParameterData::None) => println!("    {label}: <empty>"),
            Ok(LanConfigParameterData::Raw(value)) => {
                println!("    {label}: {value:02X?}")
            }
            Err(_) => println!("    {label}: invalid data"),
        }
    }
}

fn main() -> std::io::Result<()> {
    pretty_env_logger::formatted_builder()
        .parse_filters(&std::env::var("RUST_LOG").unwrap_or("info".to_string()))
        .init();

    let command = Command::parse();
    let mut ipmi = command.common.get_connection()?;

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
            ) => {
                println!("Channel 0x{raw:02X} not present (invalid channel number)");
                continue;
            }
            Err(err) => {
                log::warn!("Get Channel Info failed for 0x{raw:02X}: {err:?}");
                continue;
            }
        };

        print_channel_info(&info);
        if matches!(
            info.medium_type,
            ipmi_rs::app::ChannelMediumType::Lan802_3 | ipmi_rs::app::ChannelMediumType::OtherLan
        ) {
            print_lan_config(&mut ipmi, channel);
        }
        println!();
    }

    Ok(())
}
