use std::num::NonZeroU32;

use super::RakpErrorStatusCode;

#[derive(Debug)]
pub enum ParseError {
    NotEnoughData,
    ErrorStatusCode(ErrorStatusCode),
    UnknownErrorStatusCode(u8),
    InvalidRemoteConsoleSessionId,
}

#[derive(Debug, PartialEq)]
pub struct RakpMessage2<'a> {
    pub message_tag: u8,
    pub remote_console_session_id: NonZeroU32,
    pub managed_system_random_number: [u8; 16],
    pub managed_system_guid: [u8; 16],
    pub key_exchange_auth_code: &'a [u8],
}

impl<'a> RakpMessage2<'a> {
    pub fn from_data(data: &'a [u8]) -> Result<Self, ParseError> {
        // 4 = tag, status code, reserved bytes
        if data.len() < 4 {
            return Err(ParseError::NotEnoughData);
        }

        let message_tag = data[0];
        let status_code = data[2];

        if status_code != 0 {
            return Err(ErrorStatusCode::try_from(status_code)
                .map(ParseError::ErrorStatusCode)
                .unwrap_or(ParseError::UnknownErrorStatusCode(status_code)));
        }

        if data.len() < 40 {
            return Err(ParseError::NotEnoughData);
        }

        let remote_console_session_id =
            if let Some(v) = NonZeroU32::new(u32::from_le_bytes(data[4..8].try_into().unwrap())) {
                v
            } else {
                return Err(ParseError::InvalidRemoteConsoleSessionId);
            };

        let managed_system_random_number = data[8..24].try_into().unwrap();
        let managed_system_guid = data[24..40].try_into().unwrap();
        let key_exchange_auth_code = &data[40..];

        Ok(Self {
            message_tag,
            remote_console_session_id,
            managed_system_random_number,
            managed_system_guid,
            key_exchange_auth_code,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum ErrorStatusCode {
    Common(RakpErrorStatusCode),
    InactiveSessionId = 0x08,
    InvalidRole = 0x09,
    UnauthorizedRoleOrPrivilegeLevelRequested = 0x0A,
    InsufficientResourcesToCreateSessionAtRequestedRole = 0x0B,
    InvalidNameLength = 0x0C,
    UnauthorizedName = 0x0D,
}

impl TryFrom<u8> for ErrorStatusCode {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if let Ok(common) = TryFrom::try_from(value) {
            return Ok(ErrorStatusCode::Common(common));
        }

        let value = match value {
            0x03 => ErrorStatusCode::InactiveSessionId,
            0x09 => ErrorStatusCode::InvalidRole,
            0x0A => ErrorStatusCode::UnauthorizedRoleOrPrivilegeLevelRequested,
            0x0B => ErrorStatusCode::InsufficientResourcesToCreateSessionAtRequestedRole,
            0x0C => ErrorStatusCode::InvalidNameLength,
            0x0D => ErrorStatusCode::UnauthorizedName,
            _ => return Err(()),
        };

        Ok(value)
    }
}

#[test]
pub fn from_data() {
    let data = [
        0x00, 0x00, 0x00, 0x00, 0xa4, 0xa3, 0xa2, 0xa0, 0xe8, 0x2c, 0x00, 0x00, 0x42, 0x72, 0x00,
        0x00, 0x59, 0x18, 0x00, 0x00, 0xac, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x6b, 0x61, 0xba, 0x25, 0xc8,
        0x22, 0x4e, 0x32, 0x78, 0x63, 0x62, 0x6b, 0x30, 0x7c, 0x8d, 0xc8, 0x15, 0x22, 0x90, 0x91,
    ];

    let message = RakpMessage2::from_data(&data).unwrap();

    let expected = RakpMessage2 {
        message_tag: 0x00,
        remote_console_session_id: NonZeroU32::new(0xa0a2a3a4).unwrap(),
        managed_system_random_number: [
            0xe8, 0x2c, 0x00, 0x00, 0x42, 0x72, 0x00, 0x00, 0x59, 0x18, 0x00, 0x00, 0xac, 0x04,
            0x00, 0x00,
        ],
        managed_system_guid: [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ],
        key_exchange_auth_code: &[
            0x6b, 0x61, 0xba, 0x25, 0xc8, 0x22, 0x4e, 0x32, 0x78, 0x63, 0x62, 0x6b, 0x30, 0x7c,
            0x8d, 0xc8, 0x15, 0x22, 0x90, 0x91,
        ],
    };

    assert_eq!(expected, message);
}
