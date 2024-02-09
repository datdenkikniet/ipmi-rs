use crate::app::auth::AuthType;

mod auth;
mod md2;

use crate::connection::rmcp::plus::{PayloadType, WirePayloadType};

pub use self::auth::CalculateAuthCodeError;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnwrapEncapsulationError {
    /// There is not enough data in the packet to form a valid [`EncapsulatedMessage`].
    NotEnoughData,
    /// The auth type provided is not supported.
    UnsupportedAuthType(u8),
    /// There is a mismatch between the payload length field and the
    /// actual length of the payload.
    IncorrectPayloadLen,
    /// The auth code of the message is not correct.
    AuthcodeError,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IpmiSessionMessage {
    Ipmiv1_5 {
        auth_type: AuthType,
        session_sequence_number: u32,
        session_id: u32,
        payload: Vec<u8>,
    },
    Ipmiv2_0 {
        encrypted: bool,
        authenticated: bool,
        payload_type: PayloadType,
        session_id: u32,
        session_sequence_number: u32,
        payload: Vec<u8>,
    },
}

impl IpmiSessionMessage {
    pub fn write_data(
        &self,
        password: Option<&[u8; 16]>,
        buffer: &mut Vec<u8>,
    ) -> Result<(), CalculateAuthCodeError> {
        match self {
            IpmiSessionMessage::Ipmiv1_5 {
                auth_type,
                session_sequence_number,
                session_id,
                payload,
            } => {
                let auth_code = auth::calculate(
                    &auth_type,
                    password,
                    *session_id,
                    *session_sequence_number,
                    &payload,
                )?;

                buffer.push((*auth_type).into());
                buffer.extend_from_slice(&session_sequence_number.to_le_bytes());
                buffer.extend_from_slice(&session_id.to_le_bytes());

                if let Some(auth_code) = auth_code {
                    buffer.extend_from_slice(&auth_code);
                }

                buffer.push(payload.len() as u8);
                buffer.extend_from_slice(&payload);

                // Legacy PAD
                buffer.push(0);

                Ok(())
            }
            IpmiSessionMessage::Ipmiv2_0 {
                encrypted,
                authenticated,
                payload_type,
                session_id,
                session_sequence_number,
                payload,
            } => {
                let wire = WirePayloadType {
                    authenticated: *authenticated,
                    encrypted: *encrypted,
                    payload_type: *payload_type,
                };

                wire.write(buffer);

                Ok(())
            }
        }
    }

    fn ipmi_v1_5_from_bytes(
        password: Option<&[u8; 16]>,
        data: &[u8],
    ) -> Result<Self, UnwrapEncapsulationError> {
        if data.len() < 10 {
            return Err(UnwrapEncapsulationError::NotEnoughData);
        }

        let session_sequence = u32::from_le_bytes(data[1..5].try_into().unwrap());
        let session_id = u32::from_le_bytes(data[5..9].try_into().unwrap());

        let (auth_type, data) = match data[0] {
            0x00 => (AuthType::None, &data[9..]),
            _ => {
                if data.len() < 26 {
                    return Err(UnwrapEncapsulationError::NotEnoughData);
                }

                let auth_code: [u8; 16] = data[9..25].try_into().unwrap();

                let auth_type = match data[0] {
                    0x01 => AuthType::MD2,
                    0x02 => AuthType::MD5,
                    0x04 => AuthType::Key,
                    v => return Err(UnwrapEncapsulationError::UnsupportedAuthType(v)),
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
                    return Err(UnwrapEncapsulationError::AuthcodeError);
                }

                (auth_type, data)
            }
        };

        let data_len = data[0];
        let data = &data[1..];

        let payload = if data_len == 0 && data.is_empty() {
            Vec::new()
        }
        // Only legacy PAD
        else if data_len == 0 && data.len() == 1 {
            Vec::new()
        } else if data.len() == data_len as usize {
            data.to_vec()
        }
        // Data & legacy PAD
        else if data.len() - 1 == data_len as usize {
            data[..data.len() - 1].to_vec()
        } else {
            return Err(UnwrapEncapsulationError::IncorrectPayloadLen);
        };

        Ok(Self::Ipmiv1_5 {
            auth_type,
            session_sequence_number: session_sequence,
            session_id,
            payload,
        })
    }

    fn ipmi_v2_0_from_bytes(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < 10 {
            return Err("Not enough data");
        }

        debug_assert!(data[0] == 0x06);

        let (
            WirePayloadType {
                authenticated,
                encrypted,
                payload_type,
            },
            data,
        ) = WirePayloadType::from_data(data).ok_or("Invalid wire payload type.")?;

        let session_id = u32::from_le_bytes(data[..4].try_into().unwrap());
        let session_sequence_number = u32::from_le_bytes(data[4..8].try_into().unwrap());

        let data_len = u16::from_le_bytes(data[8..10].try_into().unwrap());
        let data = &data[10..];

        let payload = if data_len == 0 && data.is_empty() {
            Vec::new()
        } else if data.len() == data_len as usize {
            data.to_vec()
        } else {
            return Err("Payload len is not correct");
        };

        Ok(Self::Ipmiv2_0 {
            encrypted,
            payload_type,
            authenticated,
            session_id,
            session_sequence_number,
            payload,
        })
    }

    pub fn from_bytes(
        data: &[u8],
        password: Option<&[u8; 16]>,
    ) -> Result<Self, UnwrapEncapsulationError> {
        if data[0] != 0x06 {
            Self::ipmi_v1_5_from_bytes(password, data)
        } else {
            Ok(Self::ipmi_v2_0_from_bytes(data).unwrap())
        }
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

                let encapsulated =
                    IpmiSessionMessage::from_bytes(&data, Some(b"password\0\0\0\0\0\0\0\0"));

                assert_eq!(encapsulated, $then);
            }
        };
    }

    test!(
        empty_noauth,
        [0, 1, 0, 0, 0, 2, 0, 0, 0, 0],
        Ok(IpmiSessionMessage::Ipmiv1_5 {
            auth_type: AuthType::None,
            session_sequence_number: 1,
            session_id: 2,
            payload: vec![]
        })
    );

    test!(
        nonempty_noauth,
        [0, 1, 0, 0, 0, 2, 0, 0, 0, 5, 1, 2, 3, 4, 5],
        Ok(IpmiSessionMessage::Ipmiv1_5 {
            auth_type: AuthType::None,
            session_sequence_number: 1,
            session_id: 2,
            payload: vec![1, 2, 3, 4, 5]
        })
    );

    test!(
        nonempty_incorrect_len,
        [0, 1, 0, 0, 0, 2, 0, 0, 0, 5, 1, 2, 3, 4],
        Err(UnwrapEncapsulationError::IncorrectPayloadLen)
    );

    test!(
        empty_md5,
        [
            2, 1, 0, 0, 0, 2, 0, 0, 0, 152, 54, 135, 85, 190, 228, 38, 149, 133, 51, 201, 23, 232,
            140, 18, 211, 0
        ],
        Ok(IpmiSessionMessage::Ipmiv1_5 {
            auth_type: AuthType::MD5,
            session_sequence_number: 1,
            session_id: 2,
            payload: vec![]
        })
    );

    test!(
        truncated_md5,
        [2, 0, 0, 0, 1, 0, 0, 0, 2, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1],
        Err(UnwrapEncapsulationError::NotEnoughData)
    );
}
