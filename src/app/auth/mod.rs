mod get_channel_authentication_capabilities;
pub use get_channel_authentication_capabilities::{
    Channel, ChannelAuthenticationCapabilities, GetChannelAuthenticationCapabilities,
    PrivilegeLevel,
};

mod get_session_challenge;
pub use get_session_challenge::{GetSessionChallenge, SessionChallenge};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AuthType {
    None,
    MD2,
    MD5,
    Key,
}

impl TryFrom<u8> for AuthType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let value = match value {
            0 => Self::None,
            0x01 => Self::MD2,
            0x02 => Self::MD5,
            0x04 => Self::Key,
            _ => return Err(()),
        };

        Ok(value)
    }
}

impl From<AuthType> for u8 {
    fn from(value: AuthType) -> Self {
        match value {
            AuthType::None => 0x00,
            AuthType::MD2 => 0x01,
            AuthType::MD5 => 0x02,
            AuthType::Key => 0x04,
        }
    }
}
