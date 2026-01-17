use crate::common;
use crate::parse::{
    parse_ip_source, parse_ipv4, parse_ipv6, parse_ipv6_ipv4_enables, parse_mac, parse_u24,
    parse_u8,
};
use crate::types::{ConfigInput, LanConfigInput};

use ipmi_rs::{
    connection::{Channel, CompletionErrorCode},
    transport::{
        GetLanConfigParameters, LanConfigParameter, LanConfigParameterRequest,
        SetLanConfigParameters,
    },
    IpmiError,
};

pub fn apply_config_file(
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

    if let Some(enables) = config.ipv6_ipv4_addressing_enables.as_deref() {
        if let Some(value) = parse_ipv6_ipv4_enables(enables) {
            let _ = set_param(
                ipmi,
                channel,
                LanConfigParameter::Ipv6Ipv4AddressingEnables,
                LanConfigParameterRequest::Ipv6Ipv4AddressingEnables(value),
            );
        } else {
            log::warn!("Invalid IPv6/IPv4 enables: {enables}");
        }
    }

    if let Some(value) = config.ipv6_header_static_traffic_class.as_deref() {
        if let Some(byte) = parse_u8(value) {
            let _ = set_param(
                ipmi,
                channel,
                LanConfigParameter::Ipv6HeaderStaticTrafficClass,
                LanConfigParameterRequest::Ipv6HeaderStaticTrafficClass(byte),
            );
        } else {
            log::warn!("Invalid IPv6 traffic class: {value}");
        }
    }

    if let Some(value) = config.ipv6_header_static_hop_limit.as_deref() {
        if let Some(byte) = parse_u8(value) {
            let _ = set_param(
                ipmi,
                channel,
                LanConfigParameter::Ipv6HeaderStaticHopLimit,
                LanConfigParameterRequest::Ipv6HeaderStaticHopLimit(byte),
            );
        } else {
            log::warn!("Invalid IPv6 hop limit: {value}");
        }
    }

    if let Some(value) = config.ipv6_header_flow_label.as_deref() {
        if let Some(bytes) = parse_u24(value) {
            let _ = set_param(
                ipmi,
                channel,
                LanConfigParameter::Ipv6HeaderFlowLabel,
                LanConfigParameterRequest::Raw(bytes),
            );
        } else {
            log::warn!("Invalid IPv6 flow label: {value}");
        }
    }

    if let Some(addresses) = config.ipv6_static_addresses.as_ref() {
        for entry in addresses {
            if let Some(address) = parse_ipv6(&entry.address) {
                let enabled = entry.enabled.unwrap_or(true);
                let source_type = entry.source_type.unwrap_or(0);
                let status = entry.status.unwrap_or(0);
                let _ = set_param(
                    ipmi,
                    channel,
                    LanConfigParameter::Ipv6StaticAddresses,
                    LanConfigParameterRequest::Ipv6StaticAddress {
                        set_selector: entry.set_selector,
                        enabled,
                        source_type,
                        address,
                        prefix_length: entry.prefix_length,
                        status,
                    },
                );
            } else {
                log::warn!("Invalid IPv6 address: {}", entry.address);
            }
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
