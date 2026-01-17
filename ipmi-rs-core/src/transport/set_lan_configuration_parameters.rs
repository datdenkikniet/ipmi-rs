use crate::connection::{Channel, IpmiCommand, Message, NetFn, NotEnoughData};

use super::{Ipv4Address, Ipv6Address, Ipv6Ipv4Enables, LanConfigParameter, MacAddress};

/// Set LAN Configuration Parameters command.
///
/// Reference: IPMI 2.0 Specification, Table 23-2.
#[derive(Clone, Debug)]
pub struct SetLanConfigParameters {
    channel: Channel,
    parameter: LanConfigParameter,
    data: Vec<u8>,
}

impl SetLanConfigParameters {
    /// Create a new Set LAN Configuration Parameters command.
    pub fn new(channel: Channel, parameter: LanConfigParameter, data: Vec<u8>) -> Self {
        Self {
            channel,
            parameter,
            data,
        }
    }

    /// Create a Set LAN Configuration Parameters command from a typed request.
    pub fn from_request(
        channel: Channel,
        parameter: LanConfigParameter,
        request: LanConfigParameterRequest,
    ) -> Self {
        Self::new(channel, parameter, request.to_bytes())
    }
}

impl From<SetLanConfigParameters> for Message {
    fn from(value: SetLanConfigParameters) -> Self {
        let channel = value.channel.value() & 0x0F;
        let mut payload = Vec::with_capacity(2 + value.data.len());
        payload.push(channel);
        payload.push(value.parameter.value());
        payload.extend_from_slice(&value.data);
        Message::new_request(NetFn::Transport, 0x01, payload)
    }
}

impl IpmiCommand for SetLanConfigParameters {
    type Output = ();
    type Error = NotEnoughData;

    fn parse_success_response(_: &[u8]) -> Result<Self::Output, Self::Error> {
        Ok(())
    }
}

/// LAN configuration parameter request payloads.
#[derive(Clone, Debug, PartialEq)]
pub enum LanConfigParameterRequest {
    SetInProgress(u8),
    IpAddress(Ipv4Address),
    IpAddressSource(u8),
    MacAddress(MacAddress),
    SubnetMask(Ipv4Address),
    DefaultGatewayAddress(Ipv4Address),
    DefaultGatewayMacAddress(MacAddress),
    BackupGatewayAddress(Ipv4Address),
    BackupGatewayMacAddress(MacAddress),
    Ipv6Ipv4AddressingEnables(Ipv6Ipv4Enables),
    Ipv6HeaderStaticTrafficClass(u8),
    Ipv6HeaderStaticHopLimit(u8),
    Ipv6StaticAddress {
        set_selector: u8,
        enabled: bool,
        source_type: u8,
        address: Ipv6Address,
        prefix_length: u8,
        status: u8,
    },
    Raw(Vec<u8>),
}

impl LanConfigParameterRequest {
    /// Serialize a parameter request into raw bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            LanConfigParameterRequest::SetInProgress(value) => vec![*value],
            LanConfigParameterRequest::IpAddress(value)
            | LanConfigParameterRequest::SubnetMask(value)
            | LanConfigParameterRequest::DefaultGatewayAddress(value)
            | LanConfigParameterRequest::BackupGatewayAddress(value) => value.0.to_vec(),
            LanConfigParameterRequest::IpAddressSource(value) => vec![*value],
            LanConfigParameterRequest::MacAddress(value)
            | LanConfigParameterRequest::DefaultGatewayMacAddress(value)
            | LanConfigParameterRequest::BackupGatewayMacAddress(value) => value.0.to_vec(),
            LanConfigParameterRequest::Ipv6Ipv4AddressingEnables(value) => {
                vec![(*value).into()]
            }
            LanConfigParameterRequest::Ipv6HeaderStaticTrafficClass(value)
            | LanConfigParameterRequest::Ipv6HeaderStaticHopLimit(value) => vec![*value],
            LanConfigParameterRequest::Ipv6StaticAddress {
                set_selector,
                enabled,
                source_type,
                address,
                prefix_length,
                status,
            } => {
                let source = (if *enabled { 0x80 } else { 0x00 }) | (source_type & 0x0F);
                let mut bytes = Vec::with_capacity(20);
                bytes.push(*set_selector);
                bytes.push(source);
                bytes.extend_from_slice(&address.0);
                bytes.push(*prefix_length);
                bytes.push(*status);
                bytes
            }
            LanConfigParameterRequest::Raw(bytes) => bytes.clone(),
        }
    }
}
