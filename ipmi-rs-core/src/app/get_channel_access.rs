use crate::connection::{Channel, IpmiCommand, Message, NetFn, NotEnoughData};

/// The Get Channel Access command.
///
/// This command is used to return whether a given channel is enabled or disabled,
/// whether alerting is enabled or disabled for the entire channel, and under what
/// system modes the channel can be accessed.
///
/// Reference: IPMI 2.0 Specification, Table 22-28
pub struct GetChannelAccess {
    channel: Channel,
    access_type: ChannelAccessType,
}

impl GetChannelAccess {
    /// Create a new Get Channel Access command for `channel`.
    ///
    /// Use `access_type` to specify whether to get the non-volatile or volatile
    /// (currently active) settings.
    pub fn new(channel: Channel, access_type: ChannelAccessType) -> Self {
        Self {
            channel,
            access_type,
        }
    }

    /// Create a command to get the non-volatile (persistent) access settings.
    pub fn non_volatile(channel: Channel) -> Self {
        Self::new(channel, ChannelAccessType::NonVolatile)
    }

    /// Create a command to get the volatile (active) access settings.
    pub fn volatile(channel: Channel) -> Self {
        Self::new(channel, ChannelAccessType::Volatile)
    }
}

impl From<GetChannelAccess> for Message {
    fn from(value: GetChannelAccess) -> Self {
        let channel = value.channel.value() & 0x0F;
        let access_type = match value.access_type {
            ChannelAccessType::NonVolatile => 0x40, // 01b << 6
            ChannelAccessType::Volatile => 0x80,    // 10b << 6
        };
        Message::new_request(NetFn::App, 0x41, vec![channel, access_type])
    }
}

impl IpmiCommand for GetChannelAccess {
    type Output = ChannelAccess;
    type Error = NotEnoughData;

    fn parse_success_response(data: &[u8]) -> Result<Self::Output, Self::Error> {
        ChannelAccess::parse(data).ok_or(NotEnoughData)
    }
}

/// Whether to get non-volatile or volatile channel access settings.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChannelAccessType {
    /// Get non-volatile (persistent) settings stored in BMC.
    NonVolatile,
    /// Get volatile (currently active) settings.
    Volatile,
}

/// Channel access information returned by the BMC.
#[derive(Clone, Debug, PartialEq)]
pub struct ChannelAccess {
    /// The access mode for this channel.
    pub access_mode: ChannelAccessMode,
    /// Whether alerting is disabled for this channel.
    pub alerting_disabled: bool,
    /// Whether per-message authentication is disabled.
    pub per_msg_auth_disabled: bool,
    /// Whether user level authentication is disabled.
    pub user_level_auth_disabled: bool,
    /// The maximum privilege level allowed on this channel.
    pub privilege_level_limit: ChannelPrivilegeLevel,
}

impl ChannelAccess {
    /// Parse a `ChannelAccess` from IPMI response data.
    ///
    /// Reference: IPMI 2.0 Specification, Table 22-28
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 2 {
            return None;
        }

        let access_mode = ChannelAccessMode::from(data[0] & 0x07);
        let alerting_disabled = (data[0] & 0x20) == 0x20;
        let per_msg_auth_disabled = (data[0] & 0x10) == 0x10;
        let user_level_auth_disabled = (data[0] & 0x08) == 0x08;

        let privilege_level_limit = ChannelPrivilegeLevel::from(data[1] & 0x0F);

        Some(Self {
            access_mode,
            alerting_disabled,
            per_msg_auth_disabled,
            user_level_auth_disabled,
            privilege_level_limit,
        })
    }

    /// Returns true if the channel is disabled.
    pub fn is_disabled(&self) -> bool {
        matches!(self.access_mode, ChannelAccessMode::Disabled)
    }
}

/// Channel access mode values.
///
/// Reference: IPMI 2.0 Specification, Table 6-4
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChannelAccessMode {
    /// The channel is disabled from being used to communicate with the BMC.
    Disabled,
    /// The channel is only available during pre-boot (before OS loads).
    PreBootOnly,
    /// The channel is always available for communication.
    AlwaysAvailable,
    /// The channel is in shared access mode.
    Shared,
    /// Unknown value.
    Unknown(u8),
}

impl From<u8> for ChannelAccessMode {
    fn from(value: u8) -> Self {
        match value {
            0x00 => Self::Disabled,
            0x01 => Self::PreBootOnly,
            0x02 => Self::AlwaysAvailable,
            0x03 => Self::Shared,
            v => Self::Unknown(v),
        }
    }
}

impl core::fmt::Display for ChannelAccessMode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ChannelAccessMode::Disabled => write!(f, "Disabled"),
            ChannelAccessMode::PreBootOnly => write!(f, "Pre-boot Only"),
            ChannelAccessMode::AlwaysAvailable => write!(f, "Always Available"),
            ChannelAccessMode::Shared => write!(f, "Shared"),
            ChannelAccessMode::Unknown(value) => write!(f, "Unknown (0x{value:02X})"),
        }
    }
}

/// Channel privilege level values.
///
/// Reference: IPMI 2.0 Specification, Table 22-28
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChannelPrivilegeLevel {
    /// Reserved (invalid).
    Reserved,
    /// Callback privilege level.
    Callback,
    /// User privilege level.
    User,
    /// Operator privilege level.
    Operator,
    /// Administrator privilege level.
    Administrator,
    /// OEM proprietary privilege level.
    Oem,
    /// Unknown value.
    Unknown(u8),
}

impl From<u8> for ChannelPrivilegeLevel {
    fn from(value: u8) -> Self {
        match value {
            0x00 => Self::Reserved,
            0x01 => Self::Callback,
            0x02 => Self::User,
            0x03 => Self::Operator,
            0x04 => Self::Administrator,
            0x05 => Self::Oem,
            v => Self::Unknown(v),
        }
    }
}

impl core::fmt::Display for ChannelPrivilegeLevel {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ChannelPrivilegeLevel::Reserved => write!(f, "Reserved"),
            ChannelPrivilegeLevel::Callback => write!(f, "Callback"),
            ChannelPrivilegeLevel::User => write!(f, "User"),
            ChannelPrivilegeLevel::Operator => write!(f, "Operator"),
            ChannelPrivilegeLevel::Administrator => write!(f, "Administrator"),
            ChannelPrivilegeLevel::Oem => write!(f, "OEM"),
            ChannelPrivilegeLevel::Unknown(value) => write!(f, "Unknown (0x{value:02X})"),
        }
    }
}
