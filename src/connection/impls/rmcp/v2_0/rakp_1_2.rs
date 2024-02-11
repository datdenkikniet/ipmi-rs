use std::num::NonZeroU32;

use crate::app::auth::PrivilegeLevel;

use super::AuthenticationAlgorithm;

#[derive(Debug)]
pub struct Username {
    data: [u8; 16],
    length: usize,
}

impl Username {
    /// Will truncate username to max 16 bytes
    pub fn new(data: &str) -> Option<Self> {
        let chars = data.chars().take(16);

        let mut data = [0u8; 16];
        let mut length = 0;

        for (idx, char) in chars.enumerate() {
            if char.is_ascii() && char as u32 != 0 {
                data[idx] = char as u8;
                length += 1;
            } else {
                return None;
            }
        }

        Some(Self { data, length })
    }
}

impl core::ops::Deref for Username {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.data[..self.length]
    }
}

// TODO: override debug to hide crypto info
#[derive(Debug)]
pub struct RakpMessageOne<'a> {
    pub message_tag: u8,
    pub managed_system_session_id: NonZeroU32,
    pub remote_console_random_number: &'a [u8; 16],
    pub requested_maximum_privilege_level: PrivilegeLevel,
    pub username: &'a Username,
}

impl RakpMessageOne<'_> {
    pub fn write(&self, buffer: &mut Vec<u8>) {
        // Message tag
        buffer.push(self.message_tag);

        // Three reserved bytes
        buffer.extend_from_slice(&[0x00, 0x00, 0x00]);

        // Managed system session ID
        buffer.extend_from_slice(&self.managed_system_session_id.get().to_le_bytes());

        // Remote console random number
        buffer.extend_from_slice(self.remote_console_random_number);

        // Requested maximum privilege level
        buffer.push(self.requested_maximum_privilege_level.into());

        // Two reserved bytes
        buffer.extend_from_slice(&[0x00, 0x00]);

        // User name length
        buffer.push(self.username.len() as u8);

        // User name data
        buffer.extend_from_slice(&self.username);
    }
}

#[derive(Debug)]
pub struct RakpMessageTwo<'a> {
    pub message_tag: u8,
    pub remote_console_session_id: NonZeroU32,
    pub managed_system_random_number: [u8; 16],
    pub managed_system_guid: [u8; 16],
    pub key_exchange_auth_code: &'a [u8],
}

impl<'a> RakpMessageTwo<'a> {
    pub fn from_data(data: &'a [u8]) -> Result<Self, &'static str> {
        if data.len() < 2 {
            return Err("Not enough data");
        }

        let message_tag = data[0];
        let status_code = data[2];

        if status_code != 0 {
            return Err("Status code indicates error.");
        }

        if data.len() < 40 {
            return Err("Not enough data");
        }

        let remote_console_session_id =
            if let Some(v) = NonZeroU32::new(u32::from_le_bytes(data[4..8].try_into().unwrap())) {
                v
            } else {
                return Err("Invalid remote console session ID.");
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
