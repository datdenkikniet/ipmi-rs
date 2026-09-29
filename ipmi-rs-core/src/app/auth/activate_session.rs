use core::num::NonZeroU32;

use crate::connection::{EncodeIpmiCommand, IpmiCommand, NetFn};

use super::{AuthError, AuthType, PrivilegeLevel};

#[derive(Debug, Clone)]
pub struct ActivateSession {
    pub auth_type: AuthType,
    pub maximum_privilege_level: PrivilegeLevel,
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

impl EncodeIpmiCommand for ActivateSession {
    const NETFN: NetFn = NetFn::App;
    const CMD: u8 = 0x3a;

    fn request_data_len(&self) -> usize {
        22
    }

    fn write_request_data(&self, data: &mut [u8]) {
        data[0] = self.auth_type.into();
        data[1] = self.maximum_privilege_level.into();
        data[2..18].copy_from_slice(&self.challenge_string);
        data[18..22].copy_from_slice(&self.initial_sequence_number.to_le_bytes());
    }
}

impl IpmiCommand for ActivateSession {
    type Output = BeginSessionInfo;

    type Error = AuthError;

    fn parse_success_response(data: &[u8]) -> Result<Self::Output, Self::Error> {
        if data.len() < 10 {
            return Err(AuthError::NotEnoughData);
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
