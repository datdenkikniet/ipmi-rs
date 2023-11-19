use std::num::NonZeroU32;

use crate::connection::{CompletionCode, IpmiCommand, Message, NetFn, ParseResponseError};

use super::AuthType;

#[derive(Debug, Clone, Copy)]
pub struct SessionChallenge {
    pub temporary_session_id: NonZeroU32,
    pub challenge_string: [u8; 16],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GetSessionChallenge {
    auth_type: AuthType,
    username: [u8; 16],
}

impl GetSessionChallenge {
    pub fn new(auth_type: AuthType, username: Option<&str>) -> Option<Self> {
        let bytes = username.map(|u| u.as_bytes()).unwrap_or(&[]);
        if bytes.len() > 16 {
            return None;
        }

        let mut username = [0u8; 16];
        bytes
            .iter()
            .enumerate()
            .for_each(|(idx, b)| username[idx] = *b);

        Some(Self {
            auth_type,
            username,
        })
    }

    pub fn auth_type(&self) -> AuthType {
        self.auth_type
    }

    pub fn username(&self) -> &str {
        let end = self.username.iter().take_while(|v| **v != 0).count();
        unsafe { core::str::from_utf8_unchecked(&self.username[..end]) }
    }
}

impl Into<Message> for GetSessionChallenge {
    fn into(self) -> Message {
        let mut data = vec![0u8; 17];

        data[0] = self.auth_type.into();
        data[1..].copy_from_slice(&self.username);

        Message::new_request(NetFn::App, 0x39, data)
    }
}

impl IpmiCommand for GetSessionChallenge {
    type Output = SessionChallenge;

    type Error = ();

    fn parse_response(
        completion_code: CompletionCode,
        data: &[u8],
    ) -> Result<Self::Output, ParseResponseError<Self::Error>> {
        Self::check_cc_success(completion_code)?;

        if data.len() != 20 {
            return Err(ParseResponseError::NotEnoughData);
        }

        let temporary_session_id =
            NonZeroU32::try_from(u32::from_le_bytes(data[0..4].try_into().unwrap()))
                .map_err(|_| ())?;

        let challenge_string = data[4..20].try_into().unwrap();

        Ok(SessionChallenge {
            temporary_session_id,
            challenge_string,
        })
    }
}
