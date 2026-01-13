mod common;

use clap::Parser;
use ipmi_rs::{
    app::{ChannelInfo, GetChannelInfo},
    connection::{Channel, CompletionErrorCode},
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
        println!();
    }

    Ok(())
}
