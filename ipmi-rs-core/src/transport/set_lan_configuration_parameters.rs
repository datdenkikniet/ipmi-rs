#[cfg(feature = "alloc")]
use alloc::{vec, vec::Vec};

use crate::connection::{Channel, EncodeIpmiCommand, IpmiCommand, NetFn, NotEnoughData};

use super::LanConfigParameter;
#[cfg(feature = "alloc")]
use super::{Ipv4Address, Ipv6Address, Ipv6Ipv4Enables, MacAddress};

/// Set LAN Configuration Parameters command.
///
/// Reference: IPMI 2.0 Specification, Table 23-2.
#[cfg(feature = "alloc")]
#[derive(Clone, Debug)]
pub struct SetLanConfigParameters {
    channel: Channel,
    parameter: LanConfigParameter,
    data: Vec<u8>,
}

#[cfg(feature = "alloc")]
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

#[cfg(feature = "alloc")]
impl EncodeIpmiCommand for SetLanConfigParameters {
    const NETFN: NetFn = NetFn::Transport;
    const CMD: u8 = 0x01;

    fn request_data_len(&self) -> usize {
        2 + self.data.len()
    }

    fn write_request_data(&self, data: &mut [u8]) {
        data[0] = self.channel.value() & 0x0f;
        data[1] = self.parameter.value();
        data[2..].copy_from_slice(&self.data);
    }
}

#[cfg(feature = "alloc")]
impl IpmiCommand for SetLanConfigParameters {
    type Output = ();
    type Error = NotEnoughData;

    fn parse_success_response(_: &[u8]) -> Result<Self::Output, Self::Error> {
        Ok(())
    }
}

/// A borrowed Set LAN Configuration Parameters command for allocation-free encoding.
#[derive(Clone, Copy, Debug)]
pub struct SetLanConfigParametersRef<'a> {
    channel: Channel,
    parameter: LanConfigParameter,
    data: &'a [u8],
}

impl<'a> SetLanConfigParametersRef<'a> {
    /// Create a borrowed Set LAN Configuration Parameters command.
    pub const fn new(channel: Channel, parameter: LanConfigParameter, data: &'a [u8]) -> Self {
        Self {
            channel,
            parameter,
            data,
        }
    }
}

impl EncodeIpmiCommand for SetLanConfigParametersRef<'_> {
    const NETFN: NetFn = NetFn::Transport;
    const CMD: u8 = 0x01;

    fn request_data_len(&self) -> usize {
        2 + self.data.len()
    }

    fn write_request_data(&self, data: &mut [u8]) {
        data[0] = self.channel.value() & 0x0f;
        data[1] = self.parameter.value();
        data[2..].copy_from_slice(self.data);
    }
}

impl IpmiCommand for SetLanConfigParametersRef<'_> {
    type Output = ();
    type Error = NotEnoughData;

    fn parse_success_response(_: &[u8]) -> Result<Self::Output, Self::Error> {
        Ok(())
    }
}

/// LAN configuration parameter request payloads.
#[cfg(feature = "alloc")]
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

#[cfg(feature = "alloc")]
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

#[cfg(test)]
mod tests {
    use crate::connection::{Channel, EncodeIpmiCommand, NetFn};

    use super::{LanConfigParameter, SetLanConfigParametersRef};

    #[test]
    fn borrowed_command_encodes_without_owned_data() {
        let command = SetLanConfigParametersRef::new(
            Channel::Primary,
            LanConfigParameter::IpAddress,
            &[192, 0, 2, 1],
        );
        let mut buffer = [0_u8; 6];

        let request = command.encode_request(&mut buffer).unwrap();

        assert_eq!(request.netfn(), NetFn::Transport);
        assert_eq!(request.cmd(), 0x01);
        assert_eq!(request.data(), &[0x00, 0x03, 192, 0, 2, 1]);
    }
}
