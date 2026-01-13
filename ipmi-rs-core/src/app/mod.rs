//! Definitions for IPMI app commands.

mod get_device_id;
pub use get_device_id::{DeviceId, GetDeviceId};

mod get_channel_info;
pub use get_channel_info::{
    AuxChannelInfo, ChannelInfo, ChannelMediumType, ChannelProtocolType, ChannelSessionSupport,
    GetChannelInfo,
};

pub mod auth;
