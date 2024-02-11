#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum ErrorStatusCode {
    InsufficientResourcesForSessionCreation = 0x01,
    InvalidSessionId = 0x02,
    IllegalOrUnrecognizedParameter = 0x12,
}

impl From<ErrorStatusCode> for u8 {
    fn from(value: ErrorStatusCode) -> Self {
        match value {
            ErrorStatusCode::InsufficientResourcesForSessionCreation => 0x01,
            ErrorStatusCode::InvalidSessionId => 0x02,
            ErrorStatusCode::IllegalOrUnrecognizedParameter => 0x12,
        }
    }
}

impl TryFrom<u8> for ErrorStatusCode {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let value = match value {
            0x01 => ErrorStatusCode::InsufficientResourcesForSessionCreation,
            0x02 => ErrorStatusCode::InvalidSessionId,
            0x12 => ErrorStatusCode::IllegalOrUnrecognizedParameter,
            _ => return Err(()),
        };

        Ok(value)
    }
}
