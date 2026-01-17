use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize)]
pub struct LanConfig {
    pub ip_address: Option<String>,
    pub subnet_mask: Option<String>,
    pub gateway: Option<String>,
    pub mac_address: Option<String>,
    pub ip_source: Option<String>,
    pub default_gateway_mac: Option<String>,
    pub backup_gateway: Option<String>,
    pub backup_gateway_mac: Option<String>,
    pub ipv6_ipv4_support: Option<String>,
    pub ipv6_ipv4_addressing_enables: Option<String>,
    pub ipv6_header_static_traffic_class: Option<String>,
    pub ipv6_header_static_hop_limit: Option<String>,
    pub ipv6_header_flow_label: Option<String>,
    pub ipv6_status: Option<String>,
    pub ipv6_static_addresses: Option<Vec<Ipv6AddressEntry>>,
    pub ipv6_dynamic_addresses: Option<Vec<Ipv6AddressEntry>>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct LanConfigInput {
    pub ip_address: Option<String>,
    pub subnet_mask: Option<String>,
    pub gateway: Option<String>,
    pub mac_address: Option<String>,
    pub ip_source: Option<String>,
    pub default_gateway_mac: Option<String>,
    pub backup_gateway: Option<String>,
    pub backup_gateway_mac: Option<String>,
    pub ipv6_ipv4_addressing_enables: Option<String>,
    pub ipv6_header_static_traffic_class: Option<String>,
    pub ipv6_header_static_hop_limit: Option<String>,
    pub ipv6_header_flow_label: Option<String>,
    pub ipv6_static_addresses: Option<Vec<Ipv6AddressEntryInput>>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ConfigInput {
    pub channels: Vec<ChannelInput>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ChannelInput {
    pub channel_number: u8,
    pub lan_config: LanConfigInput,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Ipv6AddressEntryInput {
    pub set_selector: u8,
    pub enabled: Option<bool>,
    pub source_type: Option<u8>,
    pub address: String,
    pub prefix_length: u8,
    pub status: Option<u8>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Ipv6AddressEntry {
    pub set_selector: u8,
    pub enabled: Option<bool>,
    pub source_type: u8,
    pub address: String,
    pub prefix_length: u8,
    pub status: u8,
    pub status_label: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ChannelConfig {
    pub channel_number: u8,
    pub lan_config: LanConfig,
}

#[derive(Clone, Debug, Serialize)]
pub struct OutputConfig {
    pub channels: Vec<ChannelConfig>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FetchError {
    Timeout,
    OutOfSync,
}

#[derive(Clone, Debug)]
pub struct FetchResult {
    pub config: LanConfig,
    pub aborted: Option<FetchError>,
}
