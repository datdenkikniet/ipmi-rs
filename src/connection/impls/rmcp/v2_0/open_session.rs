use std::num::NonZeroU32;

use crate::app::auth::PrivilegeLevel;

use super::crypto::{
    Algorithm, AuthenticationAlgorithm, ConfidentialityAlgorithm, IntegrityAlgorithm,
};

#[derive(Clone, Copy)]
pub enum AlgorithmPayload {
    Authentication(Option<AuthenticationAlgorithm>),
    Integrity(Option<IntegrityAlgorithm>),
    Confidentiality(Option<ConfidentialityAlgorithm>),
}

impl AlgorithmPayload {
    pub fn write(&self, buffer: &mut Vec<u8>) {
        let (ty, value) = match *self {
            Self::Authentication(a) => (0x00, Algorithm::into_byte(a)),
            Self::Integrity(i) => (0x01, Algorithm::into_byte(i)),
            Self::Confidentiality(c) => (0x02, Algorithm::into_byte(c)),
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
                    Algorithm::from_byte(algo).map_err(|_| "Invalid authentication algorithm")?;
                Ok(Self::Authentication(auth_algo))
            }
            0x01 => {
                let auth_algo =
                    Algorithm::from_byte(algo).map_err(|_| "Invalid integrity algorithm")?;
                Ok(Self::Integrity(auth_algo))
            }
            0x02 => {
                let auth_algo =
                    Algorithm::from_byte(algo).map_err(|_| "Invalid confidentiality algorithm")?;
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
                if self.$field.is_empty() {
                    log::debug!(
                        "Using NULL value for {} algorithm payload. BMCs may have a hard time supporting this..",
                        stringify!($map)
                    );

                    write(AlgorithmPayload::$map(None));
                } else {
                    self.$field
                        .iter()
                        .map(|v| Some(*v))
                        .map(AlgorithmPayload::$map)
                        .for_each(&mut write);
                }
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
            if let Ok(error_code) = OpenSessionResponseErrorStatusCode::try_from(status_code) {
                log::warn!("RMCP+ error occurred. Status code: '{error_code:?}'");
            }
            println!("{status_code}");
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
            AlgorithmPayload::Authentication(Some(v)) => v,
            _ => return Err("Authentication payload contained non-authentication payload type."),
        };

        let integrity_payload = match AlgorithmPayload::from_data(&data[20..28])? {
            AlgorithmPayload::Integrity(Some(v)) => v,
            _ => return Err("Integrity payload contained non-integrity payload type."),
        };

        let confidentiality_payload = match AlgorithmPayload::from_data(&data[28..36])? {
            AlgorithmPayload::Confidentiality(Some(auth)) => auth,
            _ => return Err("Confidenitality payload contained non-confidentiality payload type."),
        };

        Ok(Self {
            message_tag,
            maximum_privilege_level: max_privilege_level,
            remote_console_session_id,
            managed_system_session_id,
            authentication_payload,
            integrity_payload,
            confidentiality_payload,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OpenSessionResponseErrorStatusCode {
    InsufficientResourcesForSessionCreation = 0x01,
    InvalidSessionId = 0x02,
    InvalidPayloadType = 0x03,
    InvalidAuthenticationAlgorithm = 0x04,
    InvalidIntegrityAlgorithm = 0x05,
    InvalidConfidentialityAlgorithm = 0x10,
    NoMatchingAuthenticationPayload = 0x06,
    NoMatchingIntegrityPayload = 0x07,
    NoMatchingCipherSuite = 0x011,
    InvalidRole = 0x09,
    IllegalOrUnrecognizedParameter = 0x12,
}

impl TryFrom<u8> for OpenSessionResponseErrorStatusCode {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        use OpenSessionResponseErrorStatusCode::*;

        let value = match value {
            0x01 => InsufficientResourcesForSessionCreation,
            0x02 => InvalidSessionId,
            0x03 => InvalidPayloadType,
            0x04 => InvalidAuthenticationAlgorithm,
            0x05 => InvalidIntegrityAlgorithm,
            0x10 => InvalidConfidentialityAlgorithm,
            0x06 => NoMatchingAuthenticationPayload,
            0x07 => NoMatchingIntegrityPayload,
            0x11 => NoMatchingCipherSuite,
            0x09 => InvalidRole,
            0x12 => IllegalOrUnrecognizedParameter,
            _ => return Err(()),
        };

        Ok(value)
    }
}
