use crate::connection::{Channel, IpmiCommand, Message, NetFn, NotEnoughData};

/// The Get Channel Info command.
///
/// Reference: IPMI 2.0 Specification, Table 22-29
pub struct GetChannelInfo {
    channel: Channel,
}

impl GetChannelInfo {
    /// Create a new Get Channel Info command for `channel`.
    pub fn new(channel: Channel) -> Self {
        Self { channel }
    }
}

impl From<GetChannelInfo> for Message {
    fn from(value: GetChannelInfo) -> Self {
        let channel = value.channel.value() & 0x0F;
        Message::new_request(NetFn::App, 0x42, vec![channel])
    }
}

impl IpmiCommand for GetChannelInfo {
    type Output = ChannelInfo;
    type Error = NotEnoughData;

    fn parse_success_response(data: &[u8]) -> Result<Self::Output, Self::Error> {
        ChannelInfo::parse(data).ok_or(NotEnoughData)
    }
}

/// Channel info returned by the BMC.
#[derive(Clone, Debug, PartialEq)]
pub struct ChannelInfo {
    pub channel: Channel,
    pub medium_type: ChannelMediumType,
    pub protocol_type: ChannelProtocolType,
    pub session_support: ChannelSessionSupport,
    pub active_sessions: u8,
    pub vendor_id: u32,
    pub aux_info: AuxChannelInfo,
}

impl ChannelInfo {
    /// Parse a `ChannelInfo` from IPMI response data.
    ///
    /// Reference: IPMI 2.0 Specification, Table 22-29
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 9 {
            return None;
        }

        let channel_raw = data[0] & 0x0F;
        let channel = Channel::new(channel_raw)?;

        let medium_type = ChannelMediumType::from(data[1] & 0x7F);
        let protocol_type = ChannelProtocolType::from(data[2] & 0x1F);

        let session_support = ChannelSessionSupport::from((data[3] >> 6) & 0x03);
        let active_sessions = data[3] & 0x3F;

        let vendor_id = u32::from_le_bytes([data[4], data[5], data[6], 0]);
        let aux_info = AuxChannelInfo {
            byte1: data[7],
            byte2: data[8],
        };

        Some(Self {
            channel,
            medium_type,
            protocol_type,
            session_support,
            active_sessions,
            vendor_id,
            aux_info,
        })
    }
}

/// Auxiliary channel information bytes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AuxChannelInfo {
    pub byte1: u8,
    pub byte2: u8,
}

/// Channel medium type values.
///
/// Reference: IPMI 2.0 Specification, Table 6-3
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChannelMediumType {
    Reserved,
    Ipmb,
    IcmbV1_0,
    IcmbV0_9,
    Lan802_3,
    AsynchSerialModem,
    OtherLan,
    PciSmbus,
    SmbusV1_0,
    SmbusV2_0,
    Usb1X,
    Usb2X,
    SystemInterface,
    Oem(u8),
    ReservedValue(u8),
}

impl From<u8> for ChannelMediumType {
    fn from(value: u8) -> Self {
        match value {
            0x00 => Self::Reserved,
            0x01 => Self::Ipmb,
            0x02 => Self::IcmbV1_0,
            0x03 => Self::IcmbV0_9,
            0x04 => Self::Lan802_3,
            0x05 => Self::AsynchSerialModem,
            0x06 => Self::OtherLan,
            0x07 => Self::PciSmbus,
            0x08 => Self::SmbusV1_0,
            0x09 => Self::SmbusV2_0,
            0x0A => Self::Usb1X,
            0x0B => Self::Usb2X,
            0x0C => Self::SystemInterface,
            0x60..=0x7F => Self::Oem(value),
            _ => Self::ReservedValue(value),
        }
    }
}

impl core::fmt::Display for ChannelMediumType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ChannelMediumType::Reserved => write!(f, "Reserved"),
            ChannelMediumType::Ipmb => write!(f, "IPMB (I2C)"),
            ChannelMediumType::IcmbV1_0 => write!(f, "ICMB v1.0"),
            ChannelMediumType::IcmbV0_9 => write!(f, "ICMB v0.9"),
            ChannelMediumType::Lan802_3 => write!(f, "802.3 LAN"),
            ChannelMediumType::AsynchSerialModem => write!(f, "Asynchronous Serial/Modem"),
            ChannelMediumType::OtherLan => write!(f, "Other LAN"),
            ChannelMediumType::PciSmbus => write!(f, "PCI SMBus"),
            ChannelMediumType::SmbusV1_0 => write!(f, "SMBus v1.0/1.1"),
            ChannelMediumType::SmbusV2_0 => write!(f, "SMBus v2.0"),
            ChannelMediumType::Usb1X => write!(f, "USB 1.x (reserved)"),
            ChannelMediumType::Usb2X => write!(f, "USB 2.x (reserved)"),
            ChannelMediumType::SystemInterface => write!(f, "System Interface"),
            ChannelMediumType::Oem(value) => write!(f, "OEM (0x{value:02X})"),
            ChannelMediumType::ReservedValue(value) => write!(f, "Reserved (0x{value:02X})"),
        }
    }
}

/// Channel protocol type values.
///
/// Reference: IPMI 2.0 Specification, Table 6-2
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChannelProtocolType {
    Reserved,
    IpmbV1_0,
    IcmbV1_0,
    IpmiSmbus,
    Kcs,
    Smic,
    Bt10,
    Bt15,
    TerminalMode,
    Oem(u8),
    ReservedValue(u8),
}

impl From<u8> for ChannelProtocolType {
    fn from(value: u8) -> Self {
        match value {
            0x00 => Self::Reserved,
            0x01 => Self::IpmbV1_0,
            0x02 => Self::IcmbV1_0,
            0x04 => Self::IpmiSmbus,
            0x05 => Self::Kcs,
            0x06 => Self::Smic,
            0x07 => Self::Bt10,
            0x08 => Self::Bt15,
            0x09 => Self::TerminalMode,
            0x1C..=0x1F => Self::Oem(value),
            _ => Self::ReservedValue(value),
        }
    }
}

impl core::fmt::Display for ChannelProtocolType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ChannelProtocolType::Reserved => write!(f, "Reserved"),
            ChannelProtocolType::IpmbV1_0 => write!(f, "IPMB-1.0"),
            ChannelProtocolType::IcmbV1_0 => write!(f, "ICMB-1.0"),
            ChannelProtocolType::IpmiSmbus => write!(f, "IPMI-SMBus"),
            ChannelProtocolType::Kcs => write!(f, "KCS"),
            ChannelProtocolType::Smic => write!(f, "SMIC"),
            ChannelProtocolType::Bt10 => write!(f, "BT-10"),
            ChannelProtocolType::Bt15 => write!(f, "BT-15"),
            ChannelProtocolType::TerminalMode => write!(f, "Terminal Mode"),
            ChannelProtocolType::Oem(value) => write!(f, "OEM (0x{value:02X})"),
            ChannelProtocolType::ReservedValue(value) => write!(f, "Reserved (0x{value:02X})"),
        }
    }
}

/// Session support for a channel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChannelSessionSupport {
    Sessionless,
    SingleSession,
    MultiSession,
    SessionBased,
    Reserved(u8),
}

impl From<u8> for ChannelSessionSupport {
    fn from(value: u8) -> Self {
        match value {
            0x00 => Self::Sessionless,
            0x01 => Self::SingleSession,
            0x02 => Self::MultiSession,
            0x03 => Self::SessionBased,
            v => Self::Reserved(v),
        }
    }
}

impl core::fmt::Display for ChannelSessionSupport {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ChannelSessionSupport::Sessionless => write!(f, "Session-less"),
            ChannelSessionSupport::SingleSession => write!(f, "Single-session"),
            ChannelSessionSupport::MultiSession => write!(f, "Multi-session"),
            ChannelSessionSupport::SessionBased => write!(f, "Session-based"),
            ChannelSessionSupport::Reserved(value) => write!(f, "Reserved (0x{value:02X})"),
        }
    }
}
