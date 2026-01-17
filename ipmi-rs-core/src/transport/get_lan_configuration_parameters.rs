use crate::connection::{Channel, IpmiCommand, Message, NetFn, NotEnoughData};

/// Get LAN Configuration Parameters command.
///
/// Reference: IPMI 2.0 Specification, Table 23-3.
#[derive(Clone, Debug)]
pub struct GetLanConfigParameters {
    channel: Channel,
    parameter: LanConfigParameter,
    set_selector: u8,
    block_selector: u8,
    revision_only: bool,
}

impl GetLanConfigParameters {
    /// Create a new Get LAN Configuration Parameters command.
    pub fn new(channel: Channel, parameter: LanConfigParameter) -> Self {
        Self {
            channel,
            parameter,
            set_selector: 0,
            block_selector: 0,
            revision_only: false,
        }
    }

    /// Set the set selector used for parameters that have multiple entries.
    pub fn with_set_selector(mut self, set_selector: u8) -> Self {
        self.set_selector = set_selector;
        self
    }

    /// Set the block selector used for parameters that are paged.
    pub fn with_block_selector(mut self, block_selector: u8) -> Self {
        self.block_selector = block_selector;
        self
    }

    /// Return only the parameter revision when set to `true`.
    pub fn revision_only(mut self, revision_only: bool) -> Self {
        self.revision_only = revision_only;
        self
    }
}

impl From<GetLanConfigParameters> for Message {
    fn from(value: GetLanConfigParameters) -> Self {
        let channel = value.channel.value() & 0x0F;
        let channel = if value.revision_only {
            channel | 0x80
        } else {
            channel
        };
        Message::new_request(
            NetFn::Transport,
            0x02,
            vec![
                channel,
                value.parameter.value(),
                value.set_selector,
                value.block_selector,
            ],
        )
    }
}

impl IpmiCommand for GetLanConfigParameters {
    type Output = LanConfigParameterResponse;
    type Error = NotEnoughData;

    fn parse_success_response(data: &[u8]) -> Result<Self::Output, Self::Error> {
        if data.is_empty() {
            return Err(NotEnoughData);
        }

        Ok(LanConfigParameterResponse {
            parameter_revision: data[0],
            data: data[1..].to_vec(),
        })
    }
}

/// LAN configuration parameters.
///
/// Reference: IPMI 2.0 Specification, Table 23-4.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LanConfigParameter {
    SetInProgress,
    AuthTypeSupport,
    AuthTypeEnables,
    IpAddress,
    IpAddressSource,
    MacAddress,
    SubnetMask,
    DefaultGatewayAddress,
    DefaultGatewayMacAddress,
    BackupGatewayAddress,
    BackupGatewayMacAddress,
    Other(u8),
}

impl LanConfigParameter {
    /// Get the raw parameter selector value.
    pub fn value(&self) -> u8 {
        match self {
            LanConfigParameter::SetInProgress => 0,
            LanConfigParameter::AuthTypeSupport => 1,
            LanConfigParameter::AuthTypeEnables => 2,
            LanConfigParameter::IpAddress => 3,
            LanConfigParameter::IpAddressSource => 4,
            LanConfigParameter::MacAddress => 5,
            LanConfigParameter::SubnetMask => 6,
            LanConfigParameter::DefaultGatewayAddress => 12,
            LanConfigParameter::DefaultGatewayMacAddress => 13,
            LanConfigParameter::BackupGatewayAddress => 14,
            LanConfigParameter::BackupGatewayMacAddress => 15,
            LanConfigParameter::Other(value) => *value,
        }
    }

    /// Parse known LAN configuration parameter data.
    pub fn parse(&self, data: &[u8]) -> Result<LanConfigParameterData, NotEnoughData> {
        if data.is_empty() {
            return Ok(LanConfigParameterData::None);
        }

        match self {
            LanConfigParameter::IpAddress => Ok(LanConfigParameterData::IpAddress(
                Ipv4Address::from_slice(data)?,
            )),
            LanConfigParameter::IpAddressSource => Ok(LanConfigParameterData::IpAddressSource(
                IpAddressSource::from(data[0]),
            )),
            LanConfigParameter::MacAddress => Ok(LanConfigParameterData::MacAddress(
                MacAddress::from_slice(data)?,
            )),
            LanConfigParameter::SubnetMask => Ok(LanConfigParameterData::SubnetMask(
                Ipv4Address::from_slice(data)?,
            )),
            LanConfigParameter::DefaultGatewayAddress => Ok(
                LanConfigParameterData::DefaultGatewayAddress(Ipv4Address::from_slice(data)?),
            ),
            LanConfigParameter::DefaultGatewayMacAddress => Ok(
                LanConfigParameterData::DefaultGatewayMacAddress(MacAddress::from_slice(data)?),
            ),
            LanConfigParameter::BackupGatewayAddress => Ok(
                LanConfigParameterData::BackupGatewayAddress(Ipv4Address::from_slice(data)?),
            ),
            LanConfigParameter::BackupGatewayMacAddress => Ok(
                LanConfigParameterData::BackupGatewayMacAddress(MacAddress::from_slice(data)?),
            ),
            _ => Ok(LanConfigParameterData::Raw(data.to_vec())),
        }
    }
}

/// LAN configuration response data.
#[derive(Clone, Debug, PartialEq)]
pub struct LanConfigParameterResponse {
    pub parameter_revision: u8,
    pub data: Vec<u8>,
}

impl LanConfigParameterResponse {
    /// Parse LAN parameter data using a known parameter selector.
    pub fn parse(
        &self,
        parameter: LanConfigParameter,
    ) -> Result<LanConfigParameterData, NotEnoughData> {
        parameter.parse(&self.data)
    }
}

/// LAN configuration parameter data variants.
#[derive(Clone, Debug, PartialEq)]
pub enum LanConfigParameterData {
    None,
    IpAddress(Ipv4Address),
    IpAddressSource(IpAddressSource),
    MacAddress(MacAddress),
    SubnetMask(Ipv4Address),
    DefaultGatewayAddress(Ipv4Address),
    DefaultGatewayMacAddress(MacAddress),
    BackupGatewayAddress(Ipv4Address),
    BackupGatewayMacAddress(MacAddress),
    Raw(Vec<u8>),
}

/// IPv4 address representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ipv4Address(pub [u8; 4]);

impl Ipv4Address {
    fn from_slice(data: &[u8]) -> Result<Self, NotEnoughData> {
        if data.len() < 4 {
            return Err(NotEnoughData);
        }
        Ok(Ipv4Address([data[0], data[1], data[2], data[3]]))
    }
}

impl core::fmt::Display for Ipv4Address {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}.{}.{}", self.0[0], self.0[1], self.0[2], self.0[3])
    }
}

/// MAC address representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MacAddress(pub [u8; 6]);

impl MacAddress {
    fn from_slice(data: &[u8]) -> Result<Self, NotEnoughData> {
        if data.len() < 6 {
            return Err(NotEnoughData);
        }
        Ok(MacAddress([
            data[0], data[1], data[2], data[3], data[4], data[5],
        ]))
    }
}

impl core::fmt::Display for MacAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

/// IP address source values.
///
/// Reference: IPMI 2.0 Specification, Table 23-4, parameter #4.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IpAddressSource {
    Unspecified,
    Static,
    Dhcp,
    BiosOrSystemSoftware,
    Other,
    Reserved(u8),
}

impl From<u8> for IpAddressSource {
    fn from(value: u8) -> Self {
        match value & 0x0F {
            0x00 => Self::Unspecified,
            0x01 => Self::Static,
            0x02 => Self::Dhcp,
            0x03 => Self::BiosOrSystemSoftware,
            0x04 => Self::Other,
            v => Self::Reserved(v),
        }
    }
}

impl From<IpAddressSource> for u8 {
    fn from(value: IpAddressSource) -> Self {
        match value {
            IpAddressSource::Unspecified => 0x00,
            IpAddressSource::Static => 0x01,
            IpAddressSource::Dhcp => 0x02,
            IpAddressSource::BiosOrSystemSoftware => 0x03,
            IpAddressSource::Other => 0x04,
            IpAddressSource::Reserved(v) => v & 0x0F,
        }
    }
}

impl core::fmt::Display for IpAddressSource {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            IpAddressSource::Unspecified => write!(f, "Unspecified"),
            IpAddressSource::Static => write!(f, "Static"),
            IpAddressSource::Dhcp => write!(f, "DHCP"),
            IpAddressSource::BiosOrSystemSoftware => write!(f, "BIOS/System software"),
            IpAddressSource::Other => write!(f, "Other"),
            IpAddressSource::Reserved(v) => write!(f, "Reserved (0x{v:02X})"),
        }
    }
}
