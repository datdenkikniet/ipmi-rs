mod get_channel_authentication_capabilities;
use core::cmp::Ordering;

pub use get_channel_authentication_capabilities::{
    Channel, ChannelAuthenticationCapabilities, GetChannelAuthenticationCapabilities,
};

mod get_session_challenge;
pub use get_session_challenge::{GetSessionChallenge, SessionChallenge};

mod activate_session;
pub use activate_session::{ActivateSession, BeginSessionInfo};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AuthType {
    None,
    MD2,
    MD5,
    Key,
}

impl AuthType {
    pub fn compare_strength(me: &&Self, other: &&Self) -> Ordering {
        if me == other {
            Ordering::Equal
        } else {
            if me == &&AuthType::None {
                Ordering::Less
            } else if other == &&AuthType::None {
                Ordering::Greater
            } else if me == &&AuthType::Key {
                Ordering::Less
            } else if other == &&AuthType::Key {
                Ordering::Greater
            } else if me == &&AuthType::MD2 {
                Ordering::Less
            } else if other == &&AuthType::MD5 {
                Ordering::Greater
            } else {
                Ordering::Greater
            }
        }
    }
}

#[test]
pub fn strenght_ordering() {
    let types = [AuthType::None, AuthType::MD2, AuthType::MD5, AuthType::Key];

    let max = types.iter().max_by(AuthType::compare_strength);
    let min = types.iter().min_by(AuthType::compare_strength);

    assert_eq!(max, Some(&AuthType::MD5));
    assert_eq!(min, Some(&AuthType::None));
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PrivilegeLevel {
    Callback,
    User,
    Operator,
    Administrator,
    OemProperietary,
}

impl TryFrom<u8> for PrivilegeLevel {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let value = value & 0x0F;
        let level = match value {
            1 => Self::Callback,
            2 => Self::User,
            3 => Self::Operator,
            4 => Self::Administrator,
            5 => Self::OemProperietary,
            _ => return Err(()),
        };
        Ok(level)
    }
}

impl From<PrivilegeLevel> for u8 {
    fn from(value: PrivilegeLevel) -> Self {
        match value {
            PrivilegeLevel::Callback => 1,
            PrivilegeLevel::User => 2,
            PrivilegeLevel::Operator => 3,
            PrivilegeLevel::Administrator => 4,
            PrivilegeLevel::OemProperietary => 5,
        }
    }
}
