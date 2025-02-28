use std::num::NonZeroU32;

use crate::connection::{IpmiCommand, NetFn, Request};

use super::{AuthError, AuthType};

/// A session challenge used to establish an authenticated session.
#[derive(Debug, Clone, Copy)]
pub struct SessionChallenge {
    /// The temporary ID of the session.
    pub temporary_session_id: NonZeroU32,
    /// The data for the challenge.
    pub challenge_string: [u8; 16],
}

/// The Get Session Challenge command.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GetSessionChallenge {
    /// The auth type for which to request a session challenge.
    auth_type: AuthType,
    /// The username for which to request a session challenge.
    username: [u8; 16],
}

impl GetSessionChallenge {
    /// Create a new [`GetSessionChallenge`] command.
    ///
    /// * `auth_type`: the auth type to requests a session challenge for.
    /// * `username`: an optional username to request a session challenge for.
    ///
    /// Will return `None` if `username` is longer than 16 bytes.
    pub fn new(auth_type: AuthType, username: Option<&str>) -> Option<Self> {
        let bytes = username.map(|u| u.as_bytes()).unwrap_or(&[]);
        if bytes.len() > 16 {
            return None;
        }

        let mut username = [0u8; 16];
        username[..bytes.len()].copy_from_slice(bytes);

        Some(Self {
            auth_type,
            username,
        })
    }

    /// The auth type to request a challenge for.
    pub fn auth_type(&self) -> AuthType {
        self.auth_type
    }

    /// The username to request a session challenge for.
    ///
    // TODO: return `Option<&str>`?
    pub fn username(&self) -> &str {
        let end = self.username.iter().take_while(|v| **v != 0).count();
        unsafe { core::str::from_utf8_unchecked(&self.username[..end]) }
    }
}

impl From<GetSessionChallenge> for Request {
    fn from(value: GetSessionChallenge) -> Self {
        let mut data = vec![0u8; 17];

        data[0] = value.auth_type.into();
        data[1..].copy_from_slice(&value.username);

        Request::new_default_target(NetFn::App, 0x39, data)
    }
}

impl IpmiCommand for GetSessionChallenge {
    type Output = SessionChallenge;

    type Error = AuthError;

    fn parse_success_response(data: &[u8]) -> Result<Self::Output, Self::Error> {
        if data.len() != 20 {
            return Err(AuthError::NotEnoughData);
        }

        let temporary_session_id =
            NonZeroU32::try_from(u32::from_le_bytes(data[0..4].try_into().unwrap()))
                .map_err(|_| AuthError::InvalidZeroSession)?;

        let challenge_string = data[4..20].try_into().unwrap();

        Ok(SessionChallenge {
            temporary_session_id,
            challenge_string,
        })
    }
}
