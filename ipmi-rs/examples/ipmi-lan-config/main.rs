#[path = "../common.rs"]
mod common;

mod apply;
mod fetch;
mod parse;
mod render;
mod types;

use clap::Parser;
use ipmi_rs::{
    app::{ChannelMediumType, GetChannelInfo},
    connection::{Channel, CompletionErrorCode},
    IpmiError,
};
use types::{ChannelConfig, OutputConfig};

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
    /// Print JSON schema for the --set input and exit
    #[clap(long)]
    print_schema: bool,
    /// Print an example IPv6 static address configuration JSON and exit
    #[clap(long)]
    print_v6_example: bool,
    /// Pretty-print JSON output
    #[clap(long)]
    pretty: bool,
}

fn main() -> std::io::Result<()> {
    pretty_env_logger::formatted_builder()
        .parse_filters(&std::env::var("RUST_LOG").unwrap_or("info".to_string()))
        .init();

    let command = Command::parse();

    if command.print_schema {
        println!("{}", render::render_schema());
        return Ok(());
    }
    if command.print_v6_example {
        println!("{}", render::render_ipv6_example());
        return Ok(());
    }

    let mut ipmi = command.common.get_connection()?;

    if let Some(path) = command.set.as_deref() {
        apply::apply_config_file(&mut ipmi, path, command.force_write_all)?;
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
                if let Some(fetch_err) = fetch::classify_fetch_error(&err) {
                    fetch::log_readback_abort(fetch_err);
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

        let fetch_result = fetch::fetch_lan_config(&mut ipmi, channel);
        channels.push(ChannelConfig {
            channel_number: info.channel.value(),
            lan_config: fetch_result.config,
        });
        if let Some(err) = fetch_result.aborted {
            fetch::log_readback_abort(err);
            break;
        }
    }

    if command.pretty {
        let output = OutputConfig { channels };
        let json = serde_json::to_string_pretty(&output)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
        println!("{json}");
    } else {
        println!("{}", render::render_json(&channels));
    }

    Ok(())
}
