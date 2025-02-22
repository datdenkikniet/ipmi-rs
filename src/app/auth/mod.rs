//! IPMI authentication commands.

mod get_channel_authentication_capabilities;
use core::cmp::Ordering;

pub use get_channel_authentication_capabilities::{
    ChannelAuthenticationCapabilities, GetChannelAuthenticationCapabilities,
};

mod get_session_challenge;
pub use get_session_challenge::{GetSessionChallenge, SessionChallenge};

mod activate_session;
pub use activate_session::{ActivateSession, BeginSessionInfo};

mod get_channel_cipher_suites;
pub use get_channel_cipher_suites::{ChannelCipherSuites, CipherSuite, GetChannelCipherSuites};

/// Errors that can occur during authentication.
#[derive(Debug, Clone)]
pub enum AuthError {
    /// One of the exchanged authentication responses did not
    /// contain enough data to parse the expected message.
    NotEnoughData,
    /// A non-zero session ID was received at a stage where
    /// non-zero session numbers are not allowed.
    InvalidZeroSession,
    /// An invalid auth type was encountered.
    InvalidAuthType(u8),
    /// An unknown privilege level was encountered.
    InvalidPrivilegeLevel(u8),
}

/// The authentication type that is used.
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AuthType {
    None,
    MD2,
    MD5,
    Key,
}

impl AuthType {
    /// Compare the strength of an authentication type. [`Ordering::Greater`] means
    /// that `self` is stronger than `other`.
    pub fn compare_strength(&self, other: &Self) -> Ordering {
        if self == other {
            Ordering::Equal
        } else if self == &AuthType::None {
            Ordering::Less
        } else if other == &AuthType::None {
            Ordering::Greater
        } else if self == &AuthType::Key {
            Ordering::Less
        } else if other == &AuthType::Key {
            Ordering::Greater
        } else if self == &AuthType::MD2 {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    }
}

#[test]
fn strength_ordering_individual() {
    let gt_pairs = [
        (AuthType::Key, AuthType::None),
        (AuthType::MD2, AuthType::None),
        (AuthType::MD5, AuthType::None),
        (AuthType::MD2, AuthType::Key),
        (AuthType::MD5, AuthType::Key),
        (AuthType::MD5, AuthType::MD2),
    ];

    for (greater, lesser) in gt_pairs {
        assert_eq!(greater.compare_strength(&lesser), Ordering::Greater);
        assert_eq!(lesser.compare_strength(&greater), Ordering::Less);
    }
}

#[test]
fn strength_ordering() {
    let types = [AuthType::None, AuthType::MD2, AuthType::MD5, AuthType::Key];

    let max = types.into_iter().max_by(AuthType::compare_strength);
    let min = types.into_iter().min_by(AuthType::compare_strength);

    assert_eq!(max, Some(AuthType::MD5));
    assert_eq!(min, Some(AuthType::None));
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

/// The privilege level of a session.
#[allow(missing_docs)]
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
