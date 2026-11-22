use std::num::NonZeroU32;

use crate::connection::{CompletionErrorCode, IpmiCommand, Message, NetFn};

/// The Close Session command.
///
/// Asks the BMC to close a session so that it can release the slot. The session
/// cannot be used after it has been closed.
///
/// Reference: IPMI 2.0 Specification, section 22.19
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CloseSession {
    /// The ID of the session to close.
    pub session_id: NonZeroU32,
}

impl From<CloseSession> for Message {
    fn from(value: CloseSession) -> Self {
        Message::new_request(
            NetFn::App,
            0x3C,
            value.session_id.get().to_le_bytes().to_vec(),
        )
    }
}

impl IpmiCommand for CloseSession {
    type Output = ();

    type Error = CloseSessionError;

    fn handle_completion_code(
        completion_code: CompletionErrorCode,
        _data: &[u8],
    ) -> Option<Self::Error> {
        match completion_code {
            CompletionErrorCode::CommandSpecific(0x87) => Some(CloseSessionError::InvalidSessionId),
            _ => None,
        }
    }

    fn parse_success_response(_data: &[u8]) -> Result<Self::Output, Self::Error> {
        Ok(())
    }
}

/// Errors that can occur when executing a [`CloseSession`] command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseSessionError {
    /// The BMC rejected the session ID (completion code `0x87`).
    ///
    /// The session may already have been closed, or the ID may not belong to a
    /// session on this BMC.
    InvalidSessionId,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session_id() -> NonZeroU32 {
        NonZeroU32::new(0x0102_0304).expect("non-zero")
    }

    #[test]
    fn encodes_netfn_command_and_session_id() {
        let message = Message::from(CloseSession {
            session_id: session_id(),
        });

        assert_eq!(message.netfn(), NetFn::App);
        assert_eq!(message.cmd(), 0x3C);
        assert_eq!(message.data(), &[0x04, 0x03, 0x02, 0x01]);
    }

    #[test]
    fn maps_invalid_session_id() {
        assert_eq!(
            CloseSession::handle_completion_code(CompletionErrorCode::CommandSpecific(0x87), &[]),
            Some(CloseSessionError::InvalidSessionId)
        );
    }
}
