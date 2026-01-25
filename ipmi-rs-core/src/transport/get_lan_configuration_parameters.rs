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
    Ipv6Ipv4Support,
    Ipv6Ipv4AddressingEnables,
    Ipv6HeaderStaticTrafficClass,
    Ipv6HeaderStaticHopLimit,
    Ipv6HeaderFlowLabel,
    Ipv6Status,
    Ipv6StaticAddresses,
    Ipv6DynamicAddress,
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
            LanConfigParameter::Ipv6Ipv4Support => 50,
            LanConfigParameter::Ipv6Ipv4AddressingEnables => 51,
            LanConfigParameter::Ipv6HeaderStaticTrafficClass => 52,
            LanConfigParameter::Ipv6HeaderStaticHopLimit => 53,
            LanConfigParameter::Ipv6HeaderFlowLabel => 54,
            LanConfigParameter::Ipv6Status => 55,
            LanConfigParameter::Ipv6StaticAddresses => 56,
            LanConfigParameter::Ipv6DynamicAddress => 59,
            LanConfigParameter::Other(value) => *value,
        }
    }

    /// Parse known LAN configuration parameter data.
    pub fn parse(&self, data: &[u8]) -> Result<LanConfigParameterData, NotEnoughData> {
        use LanConfigParameterData::*;

        if data.is_empty() {
            return Ok(None);
        }

        let value = match self {
            LanConfigParameter::IpAddress => IpAddress(Ipv4Address::from_slice(data)?),
            LanConfigParameter::IpAddressSource => {
                IpAddressSource(self::IpAddressSource::from(data[0]))
            }
            LanConfigParameter::MacAddress => MacAddress(self::MacAddress::from_slice(data)?),
            LanConfigParameter::SubnetMask => SubnetMask(Ipv4Address::from_slice(data)?),
            LanConfigParameter::DefaultGatewayAddress => {
                DefaultGatewayAddress(Ipv4Address::from_slice(data)?)
            }
            LanConfigParameter::DefaultGatewayMacAddress => {
                DefaultGatewayMacAddress(self::MacAddress::from_slice(data)?)
            }
            LanConfigParameter::BackupGatewayAddress => {
                BackupGatewayAddress(Ipv4Address::from_slice(data)?)
            }
            LanConfigParameter::BackupGatewayMacAddress => {
                BackupGatewayMacAddress(self::MacAddress::from_slice(data)?)
            }
            LanConfigParameter::Ipv6Ipv4Support => {
                Ipv6Ipv4Support(self::Ipv6Ipv4Support::from(data[0]))
            }
            LanConfigParameter::Ipv6Ipv4AddressingEnables => {
                Ipv6Ipv4AddressingEnables(Ipv6Ipv4Enables::from(data[0]))
            }
            LanConfigParameter::Ipv6HeaderStaticTrafficClass => {
                Ipv6HeaderStaticTrafficClass(data[0])
            }
            LanConfigParameter::Ipv6HeaderStaticHopLimit => Ipv6HeaderStaticHopLimit(data[0]),
            LanConfigParameter::Ipv6HeaderFlowLabel => {
                Ipv6HeaderFlowLabel(self::Ipv6HeaderFlowLabel::from_slice(data)?)
            }
            LanConfigParameter::Ipv6Status => Ipv6Status(self::Ipv6Status::from_slice(data)?),
            LanConfigParameter::Ipv6StaticAddresses => {
                Ipv6StaticAddresses(Ipv6StaticAddress::from_slice(data)?)
            }
            LanConfigParameter::Ipv6DynamicAddress => {
                Ipv6DynamicAddress(self::Ipv6DynamicAddress::from_slice(data)?)
            }
            _ => Raw(data.to_vec()),
        };

        Ok(value)
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
    Ipv6Ipv4Support(Ipv6Ipv4Support),
    Ipv6Ipv4AddressingEnables(Ipv6Ipv4Enables),
    Ipv6HeaderStaticTrafficClass(u8),
    Ipv6HeaderStaticHopLimit(u8),
    Ipv6HeaderFlowLabel(Ipv6HeaderFlowLabel),
    Ipv6Status(Ipv6Status),
    Ipv6StaticAddresses(Ipv6StaticAddress),
    Ipv6DynamicAddress(Ipv6DynamicAddress),
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

/// IPv6 address representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ipv6Address(pub [u8; 16]);

impl Ipv6Address {
    fn from_slice(data: &[u8]) -> Result<Self, NotEnoughData> {
        if data.len() < 16 {
            return Err(NotEnoughData);
        }
        let mut buf = [0u8; 16];
        buf.copy_from_slice(&data[..16]);
        Ok(Ipv6Address(buf))
    }
}

impl core::fmt::Display for Ipv6Address {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let addr = std::net::Ipv6Addr::from(self.0);
        write!(f, "{addr}")
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

/// IPv6/IPv4 support capabilities.
///
/// Reference: IPMI 2.0 Specification, Table 23-4, parameter #50.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ipv6Ipv4Support {
    pub ipv6_alerting_supported: bool,
    pub dual_stack_supported: bool,
    pub ipv6_only_supported: bool,
}

impl From<u8> for Ipv6Ipv4Support {
    fn from(value: u8) -> Self {
        Self {
            ipv6_alerting_supported: (value & 0x04) == 0x04,
            dual_stack_supported: (value & 0x02) == 0x02,
            ipv6_only_supported: (value & 0x01) == 0x01,
        }
    }
}

/// IPv6/IPv4 addressing enables.
///
/// Reference: IPMI 2.0 Specification, Table 23-4, parameter #51.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ipv6Ipv4Enables {
    Ipv6Disabled,
    Ipv6Only,
    Ipv6Ipv4Simultaneous,
    Reserved(u8),
}

impl From<u8> for Ipv6Ipv4Enables {
    fn from(value: u8) -> Self {
        match value {
            0x00 => Self::Ipv6Disabled,
            0x01 => Self::Ipv6Only,
            0x02 => Self::Ipv6Ipv4Simultaneous,
            v => Self::Reserved(v),
        }
    }
}

impl From<Ipv6Ipv4Enables> for u8 {
    fn from(value: Ipv6Ipv4Enables) -> Self {
        match value {
            Ipv6Ipv4Enables::Ipv6Disabled => 0x00,
            Ipv6Ipv4Enables::Ipv6Only => 0x01,
            Ipv6Ipv4Enables::Ipv6Ipv4Simultaneous => 0x02,
            Ipv6Ipv4Enables::Reserved(v) => v,
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

impl core::fmt::Display for Ipv6Ipv4Enables {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Ipv6Ipv4Enables::Ipv6Disabled => write!(f, "IPv6 disabled"),
            Ipv6Ipv4Enables::Ipv6Only => write!(f, "IPv6 only"),
            Ipv6Ipv4Enables::Ipv6Ipv4Simultaneous => write!(f, "IPv6/IPv4 simultaneous"),
            Ipv6Ipv4Enables::Reserved(v) => write!(f, "Reserved (0x{v:02X})"),
        }
    }
}

/// IPv6 header flow label (20-bit).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ipv6HeaderFlowLabel(pub u32);

impl Ipv6HeaderFlowLabel {
    fn from_slice(data: &[u8]) -> Result<Self, NotEnoughData> {
        if data.len() < 3 {
            return Err(NotEnoughData);
        }
        let raw = ((data[0] as u32) << 16) | ((data[1] as u32) << 8) | (data[2] as u32);
        Ok(Ipv6HeaderFlowLabel(raw & 0x000F_FFFF))
    }
}

/// IPv6 status capabilities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ipv6Status {
    pub static_address_max: u8,
    pub dynamic_address_max: u8,
    pub slaac_supported: bool,
    pub dhcpv6_supported: bool,
}

impl Ipv6Status {
    fn from_slice(data: &[u8]) -> Result<Self, NotEnoughData> {
        if data.len() < 3 {
            return Err(NotEnoughData);
        }
        Ok(Ipv6Status {
            static_address_max: data[0],
            dynamic_address_max: data[1],
            slaac_supported: (data[2] & 0x02) == 0x02,
            dhcpv6_supported: (data[2] & 0x01) == 0x01,
        })
    }
}

/// IPv6 static address entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ipv6StaticAddress {
    pub set_selector: u8,
    pub enabled: bool,
    pub source_type: u8,
    pub address: Ipv6Address,
    pub prefix_length: u8,
    pub status: u8,
}

impl Ipv6StaticAddress {
    fn from_slice(data: &[u8]) -> Result<Self, NotEnoughData> {
        if data.len() < 20 {
            return Err(NotEnoughData);
        }

        let set_selector = data[0];
        let source_raw = data[1];
        let enabled = (source_raw & 0x80) == 0x80;
        let source_type = source_raw & 0x0F;

        let address = Ipv6Address::from_slice(&data[2..18])?;
        let prefix_length = data[18];
        let status = data[19];

        Ok(Ipv6StaticAddress {
            set_selector,
            enabled,
            source_type,
            address,
            prefix_length,
            status,
        })
    }
}

/// IPv6 dynamic address entry (SLAAC/DHCPv6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ipv6DynamicAddress {
    pub set_selector: u8,
    pub source_type: u8,
    pub address: Ipv6Address,
    pub prefix_length: u8,
    pub status: u8,
}

impl Ipv6DynamicAddress {
    fn from_slice(data: &[u8]) -> Result<Self, NotEnoughData> {
        if data.len() < 20 {
            return Err(NotEnoughData);
        }

        let set_selector = data[0];
        let source_type = data[1] & 0x0F;
        let address = Ipv6Address::from_slice(&data[2..18])?;
        let prefix_length = data[18];
        let status = data[19];

        Ok(Ipv6DynamicAddress {
            set_selector,
            source_type,
            address,
            prefix_length,
            status,
        })
    }
}
