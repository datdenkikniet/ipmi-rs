use crate::connection::{CompletionErrorCode, IpmiCommand, Message, NetFn};

use super::PrivilegeLevel;

/// The Set Session Privilege Level command.
///
/// A session starts at User level when it is activated. This command raises the
/// privilege level of the current session, up to the maximum that was
/// negotiated during activation.
///
/// Reference: IPMI 2.0 Specification, section 22.18
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SetSessionPrivilegeLevel {
    /// The privilege level that this command requests.
    pub privilege_level: PrivilegeLevel,
}

impl From<SetSessionPrivilegeLevel> for Message {
    fn from(value: SetSessionPrivilegeLevel) -> Self {
        Message::new_request(NetFn::App, 0x3B, vec![u8::from(value.privilege_level)])
    }
}

impl IpmiCommand for SetSessionPrivilegeLevel {
    type Output = PrivilegeLevel;

    type Error = SetSessionPrivilegeLevelError;

    fn handle_completion_code(
        completion_code: CompletionErrorCode,
        _data: &[u8],
    ) -> Option<Self::Error> {
        match completion_code {
            CompletionErrorCode::CommandSpecific(0x80) => {
                Some(SetSessionPrivilegeLevelError::LevelNotAvailableForUser)
            }
            CompletionErrorCode::CommandSpecific(0x81) => {
                Some(SetSessionPrivilegeLevelError::ExceedsPrivilegeLimit)
            }
            CompletionErrorCode::CommandSpecific(0x82) => {
                Some(SetSessionPrivilegeLevelError::CannotDisableUserLevelAuthentication)
            }
            _ => None,
        }
    }

    fn parse_success_response(data: &[u8]) -> Result<Self::Output, Self::Error> {
        let level = data
            .first()
            .copied()
            .ok_or(SetSessionPrivilegeLevelError::NotEnoughData)?;

        PrivilegeLevel::try_from(level)
            .map_err(|_| SetSessionPrivilegeLevelError::InvalidLevel(level))
    }
}

/// Errors that can occur when executing a [`SetSessionPrivilegeLevel`] command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetSessionPrivilegeLevelError {
    /// The response did not contain the current privilege level.
    NotEnoughData,
    /// The BMC returned an unknown privilege level.
    InvalidLevel(u8),
    /// The requested level is not available for this user (completion code
    /// `0x80`).
    LevelNotAvailableForUser,
    /// The requested level exceeds the channel and/or user privilege limit
    /// (completion code `0x81`).
    ExceedsPrivilegeLimit,
    /// The BMC does not allow disabling User Level authentication (completion
    /// code `0x82`).
    CannotDisableUserLevelAuthentication,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_each_privilege_level() {
        let levels = [
            (PrivilegeLevel::Callback, 0x01),
            (PrivilegeLevel::User, 0x02),
            (PrivilegeLevel::Operator, 0x03),
            (PrivilegeLevel::Administrator, 0x04),
            (PrivilegeLevel::OemProperietary, 0x05),
        ];

        for (level, expected) in levels {
            let command = SetSessionPrivilegeLevel {
                privilege_level: level,
            };
            let message = Message::from(command);

            assert_eq!(message.netfn(), NetFn::App);
            assert_eq!(message.cmd(), 0x3B);
            assert_eq!(message.data(), &[expected]);
        }
    }

    #[test]
    fn parses_the_current_privilege_level() {
        let level = SetSessionPrivilegeLevel::parse_success_response(&[0x04]).unwrap();

        assert_eq!(level, PrivilegeLevel::Administrator);
    }

    #[test]
    fn rejects_a_short_response() {
        let error = SetSessionPrivilegeLevel::parse_success_response(&[]).unwrap_err();

        assert_eq!(error, SetSessionPrivilegeLevelError::NotEnoughData);
    }

    #[test]
    fn rejects_an_invalid_privilege_level() {
        let error = SetSessionPrivilegeLevel::parse_success_response(&[0x00]).unwrap_err();

        assert_eq!(error, SetSessionPrivilegeLevelError::InvalidLevel(0x00));
    }

    #[test]
    fn maps_command_specific_completion_codes() {
        let cases = [
            (
                0x80,
                SetSessionPrivilegeLevelError::LevelNotAvailableForUser,
            ),
            (0x81, SetSessionPrivilegeLevelError::ExceedsPrivilegeLimit),
            (
                0x82,
                SetSessionPrivilegeLevelError::CannotDisableUserLevelAuthentication,
            ),
        ];

        for (code, expected) in cases {
            assert_eq!(
                SetSessionPrivilegeLevel::handle_completion_code(
                    CompletionErrorCode::CommandSpecific(code),
                    &[],
                ),
                Some(expected)
            );
        }
    }
}
