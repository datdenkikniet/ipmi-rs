#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ErrorStatusCode {
    InsufficientResourcesForSessionCreation,
    InvalidSessionId,
    IllegalOrUnrecognizedParameter,
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
