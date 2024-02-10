use std::num::NonZeroU32;

use crate::app::auth::PrivilegeLevel;

use super::{AuthenticationAlgorithm, ConfidentialityAlgorithm, IntegrityAlgorithm};

#[derive(Clone, Copy)]
pub enum AlgorithmPayload {
    Authentication(AuthenticationAlgorithm),
    Integrity(IntegrityAlgorithm),
    Confidentiality(ConfidentialityAlgorithm),
}

impl AlgorithmPayload {
    pub fn write(&self, buffer: &mut Vec<u8>) {
        let (ty, value) = match *self {
            AlgorithmPayload::Authentication(a) => (0x00, a.into()),
            AlgorithmPayload::Integrity(i) => (0x01, i.into()),
            AlgorithmPayload::Confidentiality(c) => (0x02, c.into()),
        };

        // Assert valid value
        assert!((value & 0xC0) == 0);

        // Type
        buffer.push(ty);

        // reserved data
        buffer.extend_from_slice(&[0x00, 0x00]);

        // Payload len
        buffer.push(0x08);

        // Authentication algorithm
        buffer.push(value);

        // Reserved data
        buffer.extend_from_slice(&[0x00, 0x00, 0x00]);
    }

    pub fn from_data(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() != 8 {
            return Err("Incorrect amount of data");
        }

        let ty = data[0];
        let payload_len = data[3];

        if payload_len != 8 {
            return Err("Incorrect payload len field");
        }

        let algo = data[4];

        match ty {
            0x00 => {
                let auth_algo =
                    TryFrom::try_from(algo).map_err(|_| "Invalid authentication algorithm")?;
                Ok(Self::Authentication(auth_algo))
            }
            0x01 => {
                let auth_algo =
                    TryFrom::try_from(algo).map_err(|_| "Invalid integrity algorithm")?;
                Ok(Self::Integrity(auth_algo))
            }
            0x02 => {
                let auth_algo =
                    TryFrom::try_from(algo).map_err(|_| "Invalid confidentiality algorithm")?;
                Ok(Self::Confidentiality(auth_algo))
            }
            _ => Err("Invalid payload type"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct OpenSessionRequest {
    pub message_tag: u8,
    pub requested_max_privilege: Option<PrivilegeLevel>,
    pub remote_console_session_id: u32,
    pub authentication_algorithms: Vec<AuthenticationAlgorithm>,
    pub integrity_algorithms: Vec<IntegrityAlgorithm>,
    pub confidentiality_algorithms: Vec<ConfidentialityAlgorithm>,
}

impl OpenSessionRequest {
    pub fn write_data(&self, buffer: &mut Vec<u8>) {
        buffer.push(self.message_tag);
        buffer.push(self.requested_max_privilege.map(Into::into).unwrap_or(0));

        // Two reserved bytes
        // These two bytes cause no response to be sent.
        // Not sending them causes an error response: figure out why!
        buffer.push(0);
        buffer.push(0);

        buffer.extend_from_slice(&self.remote_console_session_id.to_le_bytes());

        let mut write = |v| {
            AlgorithmPayload::write(&v, buffer);
        };

        macro_rules! write_algo {
            ($field:ident, $map:ident) => {
                self.$field
                    .iter()
                    .copied()
                    .map(AlgorithmPayload::$map)
                    .for_each(&mut write);
            };
        }

        write_algo!(authentication_algorithms, Authentication);
        write_algo!(integrity_algorithms, Integrity);
        write_algo!(confidentiality_algorithms, Confidentiality);
    }
}

#[derive(Debug, Clone)]
pub struct OpenSessionResponse {
    pub message_tag: u8,
    pub status_code: u8,
    pub maximum_privilege_level: PrivilegeLevel,
    pub remote_console_session_id: u32,
    pub managed_system_session_id: NonZeroU32,
    pub authentication_payload: AuthenticationAlgorithm,
    pub integrity_payload: IntegrityAlgorithm,
    pub confidentiality_payload: ConfidentialityAlgorithm,
}

impl OpenSessionResponse {
    pub fn from_data(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < 2 {
            return Err("Not enough data");
        }

        let message_tag = data[0];
        let status_code = data[1];

        if status_code != 00 {
            return Err("RMCP+ error occurred");
        }

        if data.len() != 36 {
            return Err("Incorrect amount of data");
        }

        let max_privilege_level =
            PrivilegeLevel::try_from(data[2]).map_err(|_| "Invalid privilege level")?;

        let remote_console_session_id = u32::from_le_bytes(data[4..8].try_into().unwrap());
        let managed_system_session_id = u32::from_le_bytes(data[8..12].try_into().unwrap());
        let managed_system_session_id = NonZeroU32::try_from(managed_system_session_id)
            .map_err(|_| "Invalid managed system session ID")?;

        let authentication_payload = match AlgorithmPayload::from_data(&data[12..20])? {
            AlgorithmPayload::Authentication(v) => v,
            _ => return Err("Authentication payload contained non-authentication payload type."),
        };

        let integrity_payload = match AlgorithmPayload::from_data(&data[20..28])? {
            AlgorithmPayload::Integrity(v) => v,
            _ => return Err("Integrity payload contained non-integrity payload type."),
        };

        let confidentiality_payload = match AlgorithmPayload::from_data(&data[28..36])? {
            AlgorithmPayload::Confidentiality(auth) => auth,
            _ => return Err("Confidenitality payload contained non-confidentiality payload type."),
        };

        Ok(Self {
            message_tag,
            status_code,
            maximum_privilege_level: max_privilege_level,
            remote_console_session_id,
            managed_system_session_id,
            authentication_payload,
            integrity_payload,
            confidentiality_payload,
        })
    }
}
