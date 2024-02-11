use std::num::NonZeroU32;

use super::RakpErrorStatusCode;

#[derive(Debug, Clone)]
pub struct RakpMessage3<'a> {
    pub message_tag: u8,
    pub managed_system_session_id: NonZeroU32,
    pub contents: RakpMessage3Contents<'a>,
}

impl RakpMessage3<'_> {
    pub fn write(&self, buffer: &mut Vec<u8>) {
        buffer.push(self.message_tag);

        let (status, after_id) = match &self.contents {
            RakpMessage3Contents::Failure(status) => ((*status).into(), &[][..]),
            RakpMessage3Contents::Succes(data) => (0x00, *data),
        };

        // Status code: success
        buffer.push(status);
        buffer.extend_from_slice(&[0x00, 0x00]);
        buffer.extend_from_slice(&self.managed_system_session_id.get().to_le_bytes());
        buffer.extend_from_slice(&after_id);
    }

    pub fn is_failure(&self) -> bool {
        matches!(self.contents, RakpMessage3Contents::Failure(_))
    }
}

#[derive(Debug, Clone)]

pub enum RakpMessage3Contents<'a> {
    Failure(RakpMessage3ErrorStatusCode),
    Succes(&'a [u8]),
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum RakpMessage3ErrorStatusCode {
    Common(RakpErrorStatusCode),
    InactiveSessionId = 0x08,
    UnauthorizedGuid = 0x0E,
    InvalidIntegrityCheckValue = 0x0F,
}

impl From<RakpMessage3ErrorStatusCode> for u8 {
    fn from(value: RakpMessage3ErrorStatusCode) -> Self {
        match value {
            RakpMessage3ErrorStatusCode::Common(common) => u8::from(common),
            RakpMessage3ErrorStatusCode::InactiveSessionId => 0x08,
            RakpMessage3ErrorStatusCode::UnauthorizedGuid => 0x0E,
            RakpMessage3ErrorStatusCode::InvalidIntegrityCheckValue => 0x0F,
        }
    }
}
