use std::num::NonZeroU32;

use super::RakpErrorStatusCode;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParseError {
    NotEnoughData,
    ErrorStatusCode(ErrorStatusCode),
    UnknownErrorStatusCode(u8),
    ZeroManagedSystemSessionId,
}

#[derive(Debug, Clone)]
pub struct RakpMessage4<'a> {
    pub message_tag: u8,
    pub managed_system_session_id: NonZeroU32,
    pub integrity_check_value: &'a [u8],
}

impl<'a> RakpMessage4<'a> {
    pub fn from_data(data: &'a [u8]) -> Result<Self, ParseError> {
        // 4 = tag, status code, reserved bytes
        if data.len() < 4 {
            return Err(ParseError::NotEnoughData);
        }

        let message_tag = data[0];
        let status_code = data[1];

        if status_code != 0 {
            return Err(ErrorStatusCode::try_from(status_code)
                .map(ParseError::ErrorStatusCode)
                .unwrap_or(ParseError::UnknownErrorStatusCode(status_code)));
        }

        if data.len() < 8 {
            return Err(ParseError::NotEnoughData);
        }

        let managed_system_session_id =
            if let Ok(v) = u32::from_le_bytes(data[4..8].try_into().unwrap()).try_into() {
                v
            } else {
                return Err(ParseError::ZeroManagedSystemSessionId);
            };
        let integrity_check_value = &data[8..];

        Ok(Self {
            message_tag,
            managed_system_session_id,
            integrity_check_value,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum ErrorStatusCode {
    Common(RakpErrorStatusCode),
    InactiveSessionId = 0x08,
    InvalidIntegrityCheckValue = 0x0F,
}

impl TryFrom<u8> for ErrorStatusCode {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if let Ok(common) = RakpErrorStatusCode::try_from(value) {
            Ok(Self::Common(common))
        } else if value == 0x08 {
            Ok(Self::InactiveSessionId)
        } else if value == 0x0F {
            Ok(Self::InvalidIntegrityCheckValue)
        } else {
            Err(())
        }
    }
}
