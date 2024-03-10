use std::num::NonZeroU32;

use crate::app::auth::PrivilegeLevel;

#[derive(Debug)]
pub struct Username {
    data: [u8; 16],
    length: usize,
}

impl Username {
    pub fn new_empty() -> Self {
        Self {
            data: [0u8; 16],
            length: 0,
        }
    }

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

    pub fn len(&self) -> u8 {
        self.length as u8
    }
}

impl core::ops::Deref for Username {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.data[..self.length]
    }
}

#[derive(Debug)]
pub struct RakpMessage1<'a> {
    pub message_tag: u8,
    pub managed_system_session_id: NonZeroU32,
    pub remote_console_random_number: [u8; 16],
    pub requested_maximum_privilege_level: PrivilegeLevel,
    pub username: &'a Username,
}

impl RakpMessage1<'_> {
    pub fn write(&self, buffer: &mut Vec<u8>) {
        // Message tag
        buffer.push(self.message_tag);

        // Three reserved bytes
        buffer.extend_from_slice(&[0x00, 0x00, 0x00]);

        // Managed system session ID
        buffer.extend_from_slice(&self.managed_system_session_id.get().to_le_bytes());

        // Remote console random number
        buffer.extend_from_slice(&self.remote_console_random_number);

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

#[test]
pub fn write() {
    let expected_data = [
        0x00, 0x00, 0x00, 0x00, 0x61, 0x01, 0x59, 0x2f, 0x52, 0xf0, 0x5e, 0x19, 0x70, 0x79, 0xb4,
        0xf9, 0xab, 0xec, 0x4a, 0x0e, 0x69, 0x36, 0xa9, 0xa3, 0x04, 0x00, 0x00, 0x04, 0x6a, 0x6f,
        0x6e, 0x61,
    ];

    let message = RakpMessage1 {
        message_tag: 0x00,
        managed_system_session_id: NonZeroU32::new(0x2f590161).unwrap(),
        remote_console_random_number: [
            0x52, 0xf0, 0x5e, 0x19, 0x70, 0x79, 0xb4, 0xf9, 0xab, 0xec, 0x4a, 0x0e, 0x69, 0x36,
            0xa9, 0xa3,
        ],
        requested_maximum_privilege_level: PrivilegeLevel::Administrator,
        username: &Username::new("jona").unwrap(),
    };

    let mut buffer = Vec::new();
    message.write(&mut buffer);

    assert_eq!(expected_data, buffer.as_ref());
}
