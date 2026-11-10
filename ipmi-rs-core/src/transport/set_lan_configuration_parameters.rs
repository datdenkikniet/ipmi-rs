#[cfg(feature = "alloc")]
use alloc::{vec, vec::Vec};

use crate::connection::{
    Channel, EncodeError, EncodeIpmiCommand, IpmiCommand, NetFn, NotEnoughData,
};

use super::LanConfigParameter;
use super::{Ipv4Address, Ipv6Address, Ipv6Ipv4Enables, MacAddress};

/// Set LAN Configuration Parameters command.
///
/// `T` holds the raw parameter data. Use a borrowed `&[u8]` for allocation-free
/// encoding, or a `Vec<u8>` (see [`SetLanConfigParameters::to_owned`]) for an
/// owned command.
///
/// Reference: IPMI 2.0 Specification, Table 23-2.
#[derive(Clone, Copy, Debug)]
pub struct SetLanConfigParameters<T = &'static [u8]> {
    channel: Channel,
    parameter: LanConfigParameter,
    data: T,
}

impl<T: AsRef<[u8]>> SetLanConfigParameters<T> {
    /// Create a new Set LAN Configuration Parameters command.
    pub const fn new(channel: Channel, parameter: LanConfigParameter, data: T) -> Self {
        Self {
            channel,
            parameter,
            data,
        }
    }

    /// Copy this command into one that owns its parameter data.
    #[cfg(feature = "alloc")]
    pub fn to_owned(&self) -> SetLanConfigParameters<Vec<u8>> {
        SetLanConfigParameters::new(self.channel, self.parameter, self.data.as_ref().to_vec())
    }
}

#[cfg(feature = "alloc")]
impl SetLanConfigParameters<Vec<u8>> {
    /// Create a Set LAN Configuration Parameters command from a typed request.
    pub fn from_request(
        channel: Channel,
        parameter: LanConfigParameter,
        request: LanConfigParameterRequest<'_>,
    ) -> Self {
        Self::new(channel, parameter, request.to_bytes())
    }
}

impl<T: AsRef<[u8]>> EncodeIpmiCommand for SetLanConfigParameters<T> {
    const NETFN: NetFn = NetFn::Transport;
    const CMD: u8 = 0x01;

    fn request_data_len(&self) -> usize {
        2 + self.data.as_ref().len()
    }

    fn write_request_data(&self, data: &mut [u8]) {
        data[0] = self.channel.value() & 0x0f;
        data[1] = self.parameter.value();
        data[2..].copy_from_slice(self.data.as_ref());
    }
}

impl<T: AsRef<[u8]>> IpmiCommand for SetLanConfigParameters<T> {
    type Output = ();
    type Error = NotEnoughData;

    fn parse_success_response(_: &[u8]) -> Result<Self::Output, Self::Error> {
        Ok(())
    }
}

/// LAN configuration parameter request payloads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LanConfigParameterRequest<'a> {
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
    Raw(&'a [u8]),
}

impl LanConfigParameterRequest<'_> {
    /// Number of bytes [`Self::encode`] writes.
    pub fn encoded_len(&self) -> usize {
        match self {
            Self::SetInProgress(_)
            | Self::IpAddressSource(_)
            | Self::Ipv6Ipv4AddressingEnables(_)
            | Self::Ipv6HeaderStaticTrafficClass(_)
            | Self::Ipv6HeaderStaticHopLimit(_) => 1,
            Self::IpAddress(_)
            | Self::SubnetMask(_)
            | Self::DefaultGatewayAddress(_)
            | Self::BackupGatewayAddress(_) => 4,
            Self::MacAddress(_)
            | Self::DefaultGatewayMacAddress(_)
            | Self::BackupGatewayMacAddress(_) => 6,
            Self::Ipv6StaticAddress { .. } => 20,
            Self::Raw(bytes) => bytes.len(),
        }
    }

    /// Serialize this request into the start of `buffer` and return the written bytes.
    ///
    /// # Errors
    ///
    /// Returns [`EncodeError`] if `buffer` is shorter than [`Self::encoded_len`].
    pub fn encode<'b>(&self, buffer: &'b mut [u8]) -> Result<&'b [u8], EncodeError> {
        let required = self.encoded_len();
        let available = buffer.len();
        let out = buffer.get_mut(..required).ok_or(EncodeError {
            required,
            available,
        })?;

        match self {
            Self::SetInProgress(value)
            | Self::IpAddressSource(value)
            | Self::Ipv6HeaderStaticTrafficClass(value)
            | Self::Ipv6HeaderStaticHopLimit(value) => out[0] = *value,
            Self::Ipv6Ipv4AddressingEnables(value) => out[0] = (*value).into(),
            Self::IpAddress(value)
            | Self::SubnetMask(value)
            | Self::DefaultGatewayAddress(value)
            | Self::BackupGatewayAddress(value) => out.copy_from_slice(&value.0),
            Self::MacAddress(value)
            | Self::DefaultGatewayMacAddress(value)
            | Self::BackupGatewayMacAddress(value) => out.copy_from_slice(&value.0),
            Self::Ipv6StaticAddress {
                set_selector,
                enabled,
                source_type,
                address,
                prefix_length,
                status,
            } => {
                out[0] = *set_selector;
                out[1] = (if *enabled { 0x80 } else { 0x00 }) | (source_type & 0x0F);
                out[2..18].copy_from_slice(&address.0);
                out[18] = *prefix_length;
                out[19] = *status;
            }
            Self::Raw(bytes) => out.copy_from_slice(bytes),
        }

        Ok(out)
    }

    /// Serialize this request into an owned buffer.
    #[cfg(feature = "alloc")]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![0; self.encoded_len()];
        self.encode(&mut bytes)
            .expect("buffer is sized by encoded_len");
        bytes
    }
}

#[cfg(test)]
mod tests {
    use crate::connection::{Channel, EncodeError, EncodeIpmiCommand, NetFn};

    use super::{
        Ipv6Address, LanConfigParameter, LanConfigParameterRequest, SetLanConfigParameters,
    };

    const IP_ADDRESS: &[u8] = &[192, 0, 2, 1];

    #[test]
    fn borrowed_command_encodes_without_owned_data() {
        let command = SetLanConfigParameters::new(
            Channel::Primary,
            LanConfigParameter::IpAddress,
            IP_ADDRESS,
        );
        let mut buffer = [0_u8; 6];

        let request = command.encode_request(&mut buffer).unwrap();

        assert_eq!(request.netfn(), NetFn::Transport);
        assert_eq!(request.cmd(), 0x01);
        assert_eq!(request.data(), &[0x00, 0x03, 192, 0, 2, 1]);
    }

    #[test]
    fn dynamic_length_follows_payload_and_rejects_short_buffer() {
        let command = SetLanConfigParameters::new(
            Channel::Primary,
            LanConfigParameter::IpAddress,
            IP_ADDRESS,
        );
        assert_eq!(command.request_data_len(), 6);

        let mut buffer = [0_u8; 5];
        let err = command.encode_request(&mut buffer).unwrap_err();
        assert_eq!((err.required, err.available), (6, 5));
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn owned_command_encodes_like_borrowed_command() {
        let borrowed = SetLanConfigParameters::new(
            Channel::Primary,
            LanConfigParameter::IpAddress,
            IP_ADDRESS,
        );
        let owned = borrowed.to_owned();
        let (mut borrowed_buffer, mut owned_buffer) = ([0_u8; 6], [0_u8; 6]);

        assert_eq!(
            owned.encode_request(&mut owned_buffer).unwrap(),
            borrowed.encode_request(&mut borrowed_buffer).unwrap()
        );
    }

    #[test]
    fn static_ipv6_request_encodes_spec_layout() {
        let mut address = [0_u8; 16];
        address[0] = 0x20;
        address[15] = 0x01;
        let request = LanConfigParameterRequest::Ipv6StaticAddress {
            set_selector: 2,
            enabled: true,
            source_type: 0x13,
            address: Ipv6Address(address),
            prefix_length: 64,
            status: 0,
        };
        let mut buffer = [0xFF_u8; 24];

        let bytes = request.encode(&mut buffer).unwrap();

        let mut expected = [0_u8; 20];
        expected[0] = 2;
        expected[1] = 0x83;
        expected[2..18].copy_from_slice(&address);
        expected[18] = 64;
        assert_eq!(bytes, &expected);
    }

    #[test]
    fn request_encode_rejects_short_buffer() {
        let request = LanConfigParameterRequest::Raw(&[1, 2, 3]);

        assert_eq!(
            request.encode(&mut [0_u8; 2]),
            Err(EncodeError {
                required: 3,
                available: 2,
            })
        );
    }
}
