use std::num::NonZeroU32;

use crate::app::auth::PrivilegeLevel;

use super::RakpErrorStatusCode;

use crate::connection::rmcp::v2_0::crypto::{
    AuthenticationAlgorithm, ConfidentialityAlgorithm, IntegrityAlgorithm,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlgorithmPayloadError {
    IncorrectDataLen,
    IncorrectPayloadLenValue,
    UnknownAuthAlgorithm(u8),
    UnknownIntegrityAlgorithm(u8),
    UnknownConfidentialityAlgorithm(u8),
    UnknownPayloadType(u8),
}

#[derive(Debug, Clone, Copy)]
pub enum AlgorithmPayload {
    Authentication(AuthenticationAlgorithm),
    Integrity(IntegrityAlgorithm),
    Confidentiality(ConfidentialityAlgorithm),
}

impl AlgorithmPayload {
    pub fn write(&self, null_byte: bool, buffer: &mut Vec<u8>) {
        let (ty, value) = match *self {
            Self::Authentication(a) => (0x00, u8::from(a)),
            Self::Integrity(i) => (0x01, u8::from(i)),
            Self::Confidentiality(c) => (0x02, u8::from(c)),
        };

        // Assert valid value
        assert!((value & 0xC0) == 0);

        // Type
        buffer.push(ty);

        // reserved data
        buffer.extend_from_slice(&[0x00, 0x00]);

        // Payload len OR null byte
        if null_byte {
            buffer.push(0x00)
        } else {
            buffer.push(0x08);
        }

        // Authentication algorithm
        buffer.push(value);

        // Reserved data
        buffer.extend_from_slice(&[0x00, 0x00, 0x00]);
    }

    pub fn from_data(data: &[u8]) -> Result<Self, AlgorithmPayloadError> {
        use AlgorithmPayloadError::*;

        if data.len() != 8 {
            return Err(IncorrectDataLen);
        }

        let ty = data[0];
        let payload_len = data[3];

        if payload_len != 8 {
            return Err(IncorrectPayloadLenValue);
        }

        let algo = data[4];

        match ty {
            0x00 => {
                let algo = AuthenticationAlgorithm::try_from(algo)
                    .map_err(|_| UnknownAuthAlgorithm(algo))?;
                Ok(Self::Authentication(algo))
            }
            0x01 => {
                let algo = IntegrityAlgorithm::try_from(algo)
                    .map_err(|_| UnknownIntegrityAlgorithm(algo))?;
                Ok(Self::Integrity(algo))
            }
            0x02 => {
                let algo = ConfidentialityAlgorithm::try_from(algo)
                    .map_err(|_| UnknownConfidentialityAlgorithm(algo))?;
                Ok(Self::Confidentiality(algo))
            }
            _ => Err(UnknownPayloadType(ty)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct OpenSessionRequest {
    pub message_tag: u8,
    pub requested_max_privilege: Option<PrivilegeLevel>,
    pub remote_console_session_id: NonZeroU32,
    // TODO: technically these support vectors of algorithms, but
    // the testing platform we're trying doesn't seem to support
    // that very well.
    pub authentication_algorithms: AuthenticationAlgorithm,
    pub integrity_algorithms: IntegrityAlgorithm,
    pub confidentiality_algorithms: ConfidentialityAlgorithm,
}

impl OpenSessionRequest {
    pub fn write_data(&self, buffer: &mut Vec<u8>) {
        buffer.push(self.message_tag);
        buffer.push(self.requested_max_privilege.map(Into::into).unwrap_or(0));

        // Two reserved bytes
        buffer.push(0);
        buffer.push(0);

        buffer.extend_from_slice(&self.remote_console_session_id.get().to_le_bytes());

        AlgorithmPayload::Authentication(self.authentication_algorithms).write(false, buffer);
        AlgorithmPayload::Integrity(self.integrity_algorithms).write(false, buffer);
        AlgorithmPayload::Confidentiality(self.confidentiality_algorithms).write(false, buffer);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParseSessionResponseError {
    NotEnoughData,
    HaveErrorCode(Result<OpenSessionResponseErrorStatusCode, u8>),
    ZeroRemoteConsoleSessionId,
    ZeroManagedSystemSessionId,
    InvalidPrivilegeLevel(u8),
    InvalidAlgorithmPayload,
    AuthPayloadWasNonAuthAlgorithm,
    IntegrityPayloadWasNonIntegrityAlgorithm,
    ConfidentialityPayloadWasNonConfidentialityAlgorithm,
    AlgorithmPayloadError(AlgorithmPayloadError),
}

#[derive(Debug, Clone, PartialEq)]
pub struct OpenSessionResponse {
    pub message_tag: u8,
    pub maximum_privilege_level: PrivilegeLevel,
    pub remote_console_session_id: NonZeroU32,
    pub managed_system_session_id: NonZeroU32,
    pub authentication_payload: AuthenticationAlgorithm,
    pub integrity_payload: IntegrityAlgorithm,
    pub confidentiality_payload: ConfidentialityAlgorithm,
}

impl OpenSessionResponse {
    pub fn from_data(data: &[u8]) -> Result<Self, ParseSessionResponseError> {
        use ParseSessionResponseError::*;

        if data.len() < 2 {
            return Err(NotEnoughData);
        }

        let message_tag = data[0];
        let status_code = data[1];

        if status_code != 00 {
            return Err(HaveErrorCode(
                OpenSessionResponseErrorStatusCode::try_from(status_code).map_err(|_| status_code),
            ));
        }

        if data.len() != 36 {
            return Err(NotEnoughData);
        }

        let max_privilege_level =
            PrivilegeLevel::try_from(data[2]).map_err(|_| InvalidPrivilegeLevel(data[2]))?;

        let remote_console_session_id = u32::from_le_bytes(data[4..8].try_into().unwrap());
        let remote_console_session_id =
            NonZeroU32::new(remote_console_session_id).ok_or(ZeroRemoteConsoleSessionId)?;
        let managed_system_session_id = u32::from_le_bytes(data[8..12].try_into().unwrap());
        let managed_system_session_id =
            NonZeroU32::new(managed_system_session_id).ok_or(ZeroManagedSystemSessionId)?;

        let authentication_payload =
            match AlgorithmPayload::from_data(&data[12..20]).map_err(AlgorithmPayloadError)? {
                AlgorithmPayload::Authentication(a) => a,
                _ => return Err(AuthPayloadWasNonAuthAlgorithm),
            };

        let integrity_payload =
            match AlgorithmPayload::from_data(&data[20..28]).map_err(AlgorithmPayloadError)? {
                AlgorithmPayload::Integrity(i) => i,
                _ => return Err(IntegrityPayloadWasNonIntegrityAlgorithm),
            };

        let confidentiality_payload =
            match AlgorithmPayload::from_data(&data[28..36]).map_err(AlgorithmPayloadError)? {
                AlgorithmPayload::Confidentiality(c) => c,
                _ => return Err(ConfidentialityPayloadWasNonConfidentialityAlgorithm),
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
#[repr(u8)]
pub enum OpenSessionResponseErrorStatusCode {
    CommonRakp(RakpErrorStatusCode),
    InvalidPayloadType = 0x03,
    InvalidAuthenticationAlgorithm = 0x04,
    InvalidIntegrityAlgorithm = 0x05,
    InvalidConfidentialityAlgorithm = 0x10,
    NoMatchingAuthenticationPayload = 0x06,
    NoMatchingIntegrityPayload = 0x07,
    NoMatchingCipherSuite = 0x011,
    InvalidRole = 0x09,
}

impl TryFrom<u8> for OpenSessionResponseErrorStatusCode {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        use OpenSessionResponseErrorStatusCode::*;

        if let Ok(common) = TryFrom::try_from(value) {
            return Ok(CommonRakp(common));
        }

        let value = match value {
            0x03 => InvalidPayloadType,
            0x04 => InvalidAuthenticationAlgorithm,
            0x05 => InvalidIntegrityAlgorithm,
            0x10 => InvalidConfidentialityAlgorithm,
            0x06 => NoMatchingAuthenticationPayload,
            0x07 => NoMatchingIntegrityPayload,
            0x11 => NoMatchingCipherSuite,
            0x09 => InvalidRole,
            _ => return Err(()),
        };

        Ok(value)
    }
}

#[test]
pub fn from_data() {
    let data = [
        0x00, 0x00, 0x04, 0x00, 0xa4, 0xa3, 0xa2, 0x0a, 0xe0, 0x34, 0x71, 0x4a, 0x00, 0x00, 0x00,
        0x08, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x08, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00,
        0x00, 0x08, 0x00, 0x00, 0x00, 0x00,
    ];

    let message = OpenSessionResponse::from_data(&data).unwrap();

    let expected = OpenSessionResponse {
        message_tag: 0x00,
        maximum_privilege_level: PrivilegeLevel::Administrator,
        remote_console_session_id: NonZeroU32::new(0x0aa2a3a4).unwrap(),
        managed_system_session_id: NonZeroU32::new(0x4a7134e0).unwrap(),
        authentication_payload: AuthenticationAlgorithm::RakpHmacSha1,
        integrity_payload: IntegrityAlgorithm::HmacSha1_96,
        confidentiality_payload: ConfidentialityAlgorithm::None,
    };

    assert_eq!(message, expected);
}
