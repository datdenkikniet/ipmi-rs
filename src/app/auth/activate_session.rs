use std::num::NonZeroU32;

use crate::connection::{IpmiCommand, Message, NetFn, ParseResponseError};

use super::{AuthError, AuthType, PrivilegeLevel};

#[derive(Debug, Clone)]
pub struct ActivateSession {
    pub auth_type: AuthType,
    pub maxiumum_privilege_level: PrivilegeLevel,
    pub challenge_string: [u8; 16],
    pub initial_sequence_number: u32,
}

#[derive(Debug, Clone)]
pub struct BeginSessionInfo {
    pub auth_type: AuthType,
    pub session_id: NonZeroU32,
    pub initial_sequence_number: u32,
    pub maximum_privilege_level: PrivilegeLevel,
}

impl From<ActivateSession> for Message {
    fn from(value: ActivateSession) -> Self {
        let mut data = vec![0u8; 22];

        data[0] = value.auth_type.into();
        data[1] = value.maxiumum_privilege_level.into();
        data[2..18].copy_from_slice(&value.challenge_string);
        data[18..22].copy_from_slice(&value.initial_sequence_number.to_le_bytes());

        Message::new_request(NetFn::App, 0x3A, data)
    }
}

impl IpmiCommand for ActivateSession {
    type Output = BeginSessionInfo;

    type Error = AuthError;

    fn parse_response(
        completion_code: crate::connection::CompletionCode,
        data: &[u8],
    ) -> Result<Self::Output, ParseResponseError<Self::Error>> {
        Self::check_cc_success(completion_code)?;

        if data.len() < 10 {
            return Err(ParseResponseError::NotEnoughData);
        }

        let auth_type = data[0]
            .try_into()
            .map_err(|_| AuthError::InvalidAuthType(data[0]))?;

        let session_id = NonZeroU32::try_from(u32::from_le_bytes(data[1..5].try_into().unwrap()))
            .map_err(|_| AuthError::InvalidZeroSession)?;
        let initial_sequence_number = u32::from_le_bytes(data[5..9].try_into().unwrap());
        let maximum_privilege_level = data[9]
            .try_into()
            .map_err(|_| AuthError::InvalidPrivilegeLevel(data[9]))?;

        Ok(BeginSessionInfo {
            auth_type,
            session_id,
            initial_sequence_number,
            maximum_privilege_level,
        })
    }
}
