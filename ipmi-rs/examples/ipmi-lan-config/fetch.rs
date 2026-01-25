use crate::common;
use crate::types::{FetchError, FetchResult, Ipv6AddressEntry, LanConfig};

use ipmi_rs::{
    connection::{Channel, CompletionErrorCode},
    transport::{
        GetLanConfigParameters, IpAddressSource, Ipv6DynamicAddress, Ipv6StaticAddress,
        LanConfigParameter, LanConfigParameterData,
    },
    IpmiError,
};

pub fn fetch_lan_config(ipmi: &mut common::IpmiConnectionEnum, channel: Channel) -> FetchResult {
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

    if let Err(err) = fill_param(
        &mut config.ipv6_ipv4_support,
        ipmi,
        channel,
        LanConfigParameter::Ipv6Ipv4Support,
    ) {
        return FetchResult {
            config,
            aborted: Some(err),
        };
    }
    if let Err(err) = fill_param(
        &mut config.ipv6_ipv4_addressing_enables,
        ipmi,
        channel,
        LanConfigParameter::Ipv6Ipv4AddressingEnables,
    ) {
        return FetchResult {
            config,
            aborted: Some(err),
        };
    }
    if let Err(err) = fill_param(
        &mut config.ipv6_header_static_traffic_class,
        ipmi,
        channel,
        LanConfigParameter::Ipv6HeaderStaticTrafficClass,
    ) {
        return FetchResult {
            config,
            aborted: Some(err),
        };
    }
    if let Err(err) = fill_param(
        &mut config.ipv6_header_static_hop_limit,
        ipmi,
        channel,
        LanConfigParameter::Ipv6HeaderStaticHopLimit,
    ) {
        return FetchResult {
            config,
            aborted: Some(err),
        };
    }
    if let Err(err) = fill_param(
        &mut config.ipv6_header_flow_label,
        ipmi,
        channel,
        LanConfigParameter::Ipv6HeaderFlowLabel,
    ) {
        return FetchResult {
            config,
            aborted: Some(err),
        };
    }
    if let Err(err) = fill_param(
        &mut config.ipv6_status,
        ipmi,
        channel,
        LanConfigParameter::Ipv6Status,
    ) {
        return FetchResult {
            config,
            aborted: Some(err),
        };
    }
    if let Err(err) = fill_ipv6_static_addresses(&mut config, ipmi, channel) {
        return FetchResult {
            config,
            aborted: Some(err),
        };
    }
    if let Err(err) = fill_ipv6_dynamic_addresses(&mut config, ipmi, channel) {
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

pub fn log_readback_abort(err: FetchError) {
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
        Ok(LanConfigParameterData::Ipv6Ipv4Support(value)) => Ok(Some(format!(
            "alerting={}, dual_stack={}, ipv6_only={}",
            value.ipv6_alerting_supported, value.dual_stack_supported, value.ipv6_only_supported
        ))),
        Ok(LanConfigParameterData::Ipv6Ipv4AddressingEnables(value)) => Ok(Some(value.to_string())),
        Ok(LanConfigParameterData::Ipv6HeaderStaticTrafficClass(value)) => {
            Ok(Some(format!("0x{value:02X}")))
        }
        Ok(LanConfigParameterData::Ipv6HeaderStaticHopLimit(value)) => {
            Ok(Some(format!("0x{value:02X}")))
        }
        Ok(LanConfigParameterData::Ipv6HeaderFlowLabel(value)) => {
            Ok(Some(format!("0x{:05X}", value.0)))
        }
        Ok(LanConfigParameterData::Ipv6Status(value)) => Ok(Some(format!(
            "static_max={}, dynamic_max={}, slaac={}, dhcpv6={}",
            value.static_address_max,
            value.dynamic_address_max,
            value.slaac_supported,
            value.dhcpv6_supported
        ))),
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

fn fill_ipv6_static_addresses(
    config: &mut LanConfig,
    ipmi: &mut common::IpmiConnectionEnum,
    channel: Channel,
) -> Result<(), FetchError> {
    let max_entries = 16;
    let mut entries = Vec::new();
    for set_selector in 0u8..max_entries {
        let response = match retry_get_param_with_set_selector(
            ipmi,
            channel,
            LanConfigParameter::Ipv6StaticAddresses,
            set_selector,
        )? {
            Some(response) => response,
            None => break,
        };

        let entry = match response.parse(LanConfigParameter::Ipv6StaticAddresses) {
            Ok(LanConfigParameterData::Ipv6StaticAddresses(value)) => value,
            _ => break,
        };

        if entry.set_selector != set_selector {
            break;
        }

        let formatted = format_ipv6_entry(entry);
        if !is_empty_ipv6_entry(&formatted) {
            entries.push(formatted);
        }
    }

    if !entries.is_empty() {
        config.ipv6_static_addresses = Some(entries);
    }

    Ok(())
}

fn fill_ipv6_dynamic_addresses(
    config: &mut LanConfig,
    ipmi: &mut common::IpmiConnectionEnum,
    channel: Channel,
) -> Result<(), FetchError> {
    let max_entries = 16;
    let mut entries = Vec::new();
    for set_selector in 0u8..max_entries {
        let response = match retry_get_param_with_set_selector(
            ipmi,
            channel,
            LanConfigParameter::Ipv6DynamicAddress,
            set_selector,
        )? {
            Some(response) => response,
            None => break,
        };

        let entry = match response.parse(LanConfigParameter::Ipv6DynamicAddress) {
            Ok(LanConfigParameterData::Ipv6DynamicAddress(value)) => value,
            _ => break,
        };

        if entry.set_selector != set_selector {
            break;
        }

        let formatted = format_ipv6_dynamic_entry(entry);
        if !is_empty_ipv6_entry(&formatted) {
            entries.push(formatted);
        }
    }

    if !entries.is_empty() {
        config.ipv6_dynamic_addresses = Some(entries);
    }

    Ok(())
}

fn format_ipv6_entry(entry: Ipv6StaticAddress) -> Ipv6AddressEntry {
    Ipv6AddressEntry {
        set_selector: entry.set_selector,
        enabled: Some(entry.enabled),
        source_type: entry.source_type,
        address: entry.address.to_string(),
        prefix_length: entry.prefix_length,
        status: entry.status,
        status_label: ipv6_status_label(entry.status).to_string(),
    }
}

fn format_ipv6_dynamic_entry(entry: Ipv6DynamicAddress) -> Ipv6AddressEntry {
    Ipv6AddressEntry {
        set_selector: entry.set_selector,
        enabled: None,
        source_type: entry.source_type,
        address: entry.address.to_string(),
        prefix_length: entry.prefix_length,
        status: entry.status,
        status_label: ipv6_status_label(entry.status).to_string(),
    }
}

fn ipv6_status_label(status: u8) -> &'static str {
    match status {
        0x00 => "Active",
        0x01 => "Disabled",
        0x02 => "Pending",
        0x03 => "Failed",
        0x04 => "Deprecated",
        0x05 => "Invalid",
        _ => "Reserved",
    }
}

fn is_empty_ipv6_entry(entry: &Ipv6AddressEntry) -> bool {
    let enabled = entry.enabled.unwrap_or(false);
    entry.address == "::" && entry.prefix_length == 0 && entry.status == 0 && !enabled
}

fn retry_get_param(
    ipmi: &mut common::IpmiConnectionEnum,
    channel: Channel,
    param: LanConfigParameter,
) -> Result<Option<ipmi_rs::transport::LanConfigParameterResponse>, FetchError> {
    retry_get_param_with_set_selector(ipmi, channel, param, 0)
}

fn retry_get_param_with_set_selector(
    ipmi: &mut common::IpmiConnectionEnum,
    channel: Channel,
    param: LanConfigParameter,
    set_selector: u8,
) -> Result<Option<ipmi_rs::transport::LanConfigParameterResponse>, FetchError> {
    let retry_delay = std::time::Duration::from_millis(200);
    let max_attempts = 3;

    for attempt in 0..max_attempts {
        let response = ipmi
            .send_recv(GetLanConfigParameters::new(channel, param).with_set_selector(set_selector));
        match response {
            Ok(response) => return Ok(Some(response)),
            Err(IpmiError::Failed {
                completion_code:
                    CompletionErrorCode::CommandSpecific(0x80)
                    | CompletionErrorCode::ParameterOutOfRange
                    | CompletionErrorCode::InvalidDataFieldInRequest
                    | CompletionErrorCode::RequestedDatapointNotPresent,
                ..
            })
            | Err(IpmiError::Command {
                completion_code:
                    Some(
                        CompletionErrorCode::CommandSpecific(0x80)
                        | CompletionErrorCode::ParameterOutOfRange
                        | CompletionErrorCode::InvalidDataFieldInRequest
                        | CompletionErrorCode::RequestedDatapointNotPresent,
                    ),
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

pub fn classify_fetch_error(err: &std::io::Error) -> Option<FetchError> {
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
