use crate::app::auth::AuthType;

use super::{auth, ReadError, WriteError};

#[derive(Debug, Clone, PartialEq)]

pub struct Message {
    pub auth_type: AuthType,
    pub session_sequence_number: u32,
    pub session_id: u32,
    pub payload: Vec<u8>,
}

impl Message {
    pub fn write_data(
        &self,
        password: Option<&[u8; 16]>,
        buffer: &mut Vec<u8>,
    ) -> Result<(), WriteError> {
        let auth_code = auth::calculate(
            &self.auth_type,
            password,
            self.session_id,
            self.session_sequence_number,
            &self.payload,
        )?;

        buffer.push(self.auth_type.into());
        buffer.extend_from_slice(&self.session_sequence_number.to_le_bytes());
        buffer.extend_from_slice(&self.session_id.to_le_bytes());

        if let Some(auth_code) = auth_code {
            buffer.extend_from_slice(&auth_code);
        }

        if self.payload.len() > u8::MAX as usize {
            return Err(WriteError::PayloadTooLarge(self.payload.len()));
        }

        buffer.push(self.payload.len() as u8);
        buffer.extend_from_slice(&self.payload);

        // Legacy PAD
        buffer.push(0);

        Ok(())
    }

    pub fn from_data(password: Option<&[u8; 16]>, data: &[u8]) -> Result<Self, ReadError> {
        if data.len() < 10 {
            return Err(ReadError::NotEnoughData);
        }

        let session_sequence = u32::from_le_bytes(data[1..5].try_into().unwrap());
        let session_id = u32::from_le_bytes(data[5..9].try_into().unwrap());

        let (auth_type, data) = match data[0] {
            0x00 => (AuthType::None, &data[9..]),
            _ => {
                if data.len() < 26 {
                    return Err(ReadError::NotEnoughData);
                }

                let auth_code: [u8; 16] = data[9..25].try_into().unwrap();

                let auth_type = match data[0] {
                    0x01 => AuthType::MD2,
                    0x02 => AuthType::MD5,
                    0x04 => AuthType::Key,
                    v => return Err(ReadError::UnsupportedAuthType(v)),
                };

                let data = &data[25..];

                if !auth::verify(
                    &auth_type,
                    auth_code,
                    password,
                    session_id,
                    session_sequence,
                    data,
                ) {
                    return Err(ReadError::AuthcodeError);
                }

                (auth_type, data)
            }
        };

        let data_len = data[0];
        let data = &data[1..];

        let empty = data_len == 0 && data.is_empty();
        let only_legacy_pad = data_len == 0 && data.len() == 1;

        let payload = if empty || only_legacy_pad {
            Vec::new()
        } else if data.len() == data_len as usize {
            data.to_vec()
        }
        // Data & legacy PAD
        else if data.len() - 1 == data_len as usize {
            data[..data.len() - 1].to_vec()
        } else {
            return Err(ReadError::IncorrectPayloadLen);
        };

        Ok(Self {
            auth_type,
            session_sequence_number: session_sequence,
            session_id,
            payload,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    macro_rules! test {
        ($name:ident, $data:expr, $then:expr) => {
            #[test]
            pub fn $name() {
                let data = $data;

                let encapsulated = Message::from_data(Some(b"password\0\0\0\0\0\0\0\0"), &data);

                assert_eq!(encapsulated, $then);
            }
        };
    }

    test!(
        empty_noauth,
        [0, 1, 0, 0, 0, 2, 0, 0, 0, 0],
        Ok(Message {
            auth_type: AuthType::None,
            session_sequence_number: 1,
            session_id: 2,
            payload: vec![]
        })
    );

    test!(
        nonempty_noauth,
        [0, 1, 0, 0, 0, 2, 0, 0, 0, 5, 1, 2, 3, 4, 5],
        Ok(Message {
            auth_type: AuthType::None,
            session_sequence_number: 1,
            session_id: 2,
            payload: vec![1, 2, 3, 4, 5]
        })
    );

    test!(
        nonempty_incorrect_len,
        [0, 1, 0, 0, 0, 2, 0, 0, 0, 5, 1, 2, 3, 4],
        Err(ReadError::IncorrectPayloadLen)
    );

    test!(
        empty_md5,
        [
            2, 1, 0, 0, 0, 2, 0, 0, 0, 152, 54, 135, 85, 190, 228, 38, 149, 133, 51, 201, 23, 232,
            140, 18, 211, 0
        ],
        Ok(Message {
            auth_type: AuthType::MD5,
            session_sequence_number: 1,
            session_id: 2,
            payload: vec![]
        })
    );

    test!(
        truncated_md5,
        [2, 0, 0, 0, 1, 0, 0, 0, 2, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1],
        Err(ReadError::NotEnoughData)
    );
}
