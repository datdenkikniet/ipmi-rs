use crate::app::auth::AuthType;

mod auth;
use auth::AuthExt;

pub use self::auth::CalculateAuthCodeError;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PayloadType {
    IpmiMessage,
    Sol,
    OemExplicit,
    RmcpPlusOpenSessionRequest,
    RmcpPlusOpenSessionResponse,
    RAKPMessage1,
    RAKPMessage2,
    RAKPMessage3,
    RAKPMessage4,
}

impl TryFrom<u8> for PayloadType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let value = value & 0x3F;
        let ty = match value {
            0x00 => Self::IpmiMessage,
            0x01 => Self::Sol,
            0x02 => Self::OemExplicit,
            0x10 => Self::RmcpPlusOpenSessionRequest,
            0x11 => Self::RmcpPlusOpenSessionResponse,
            0x12 => Self::RAKPMessage1,
            0x13 => Self::RAKPMessage2,
            0x14 => Self::RAKPMessage3,
            0x15 => Self::RAKPMessage4,
            _ => return Err(()),
        };

        Ok(ty)
    }
}

impl From<PayloadType> for u8 {
    fn from(value: PayloadType) -> Self {
        match value {
            PayloadType::IpmiMessage => 0x0,
            PayloadType::Sol => 0x01,
            PayloadType::OemExplicit => 0x02,
            PayloadType::RmcpPlusOpenSessionRequest => 0x10,
            PayloadType::RmcpPlusOpenSessionResponse => 0x11,
            PayloadType::RAKPMessage1 => 0x12,
            PayloadType::RAKPMessage2 => 0x13,
            PayloadType::RAKPMessage3 => 0x14,
            PayloadType::RAKPMessage4 => 0x15,
        }
    }
}

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
pub struct EncapsulatedMessage {
    pub auth_type: AuthType,
    pub session_sequence: u32,
    pub session_id: u32,
    pub payload: Vec<u8>,
}

impl EncapsulatedMessage {
    pub fn write_data(
        &self,
        password: Option<&[u8; 16]>,
        buffer: &mut Vec<u8>,
    ) -> Result<(), CalculateAuthCodeError> {
        let auth_type = self.auth_type.calculate(
            password,
            self.session_id,
            self.session_sequence,
            &self.payload,
        )?;

        buffer.push(self.auth_type.into());
        buffer.extend_from_slice(&self.session_sequence.to_le_bytes());
        buffer.extend_from_slice(&self.session_id.to_le_bytes());

        if let Some(auth_code) = auth_type {
            buffer.extend_from_slice(auth_code.as_slice());
        }

        buffer.push(self.payload.len() as u8);
        buffer.extend_from_slice(&self.payload);

        // Legacy PAD
        buffer.push(0);

        Ok(())
    }

    pub fn from_bytes(
        data: &[u8],
        password: Option<&[u8; 16]>,
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

                if !auth_type.verify(auth_code, password, session_id, session_sequence, data) {
                    return Err(UnwrapEncapsulationError::AuthcodeError);
                }

                (auth_type, &data[25..])
            }
        };

        let data_len = data[0];
        let data = &data[1..];

        let payload = if data_len == 0 && data.is_empty() {
            Vec::new()
        } else if data.len() == data_len as usize {
            data.to_vec()
        } else {
            return Err(UnwrapEncapsulationError::IncorrectPayloadLen);
        };

        let me = Self {
            auth_type,
            session_sequence,
            session_id,
            payload,
        };

        Ok(me)
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
                    EncapsulatedMessage::from_bytes(&data, Some(b"password\0\0\0\0\0\0\0\0"));

                assert_eq!(encapsulated, $then);
            }
        };
    }

    test!(
        empty_noauth,
        [0, 1, 0, 0, 0, 2, 0, 0, 0, 0],
        Ok(EncapsulatedMessage {
            auth_type: AuthType::None,
            session_sequence: 1,
            session_id: 2,
            payload: vec![]
        })
    );

    test!(
        nonempty_noauth,
        [0, 1, 0, 0, 0, 2, 0, 0, 0, 5, 1, 2, 3, 4, 5],
        Ok(EncapsulatedMessage {
            auth_type: AuthType::None,
            session_sequence: 1,
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
            2, 1, 0, 0, 0, 2, 0, 0, 0, 7, 160, 164, 43, 148, 8, 192, 45, 157, 45, 51, 53, 86, 32,
            148, 162, 0
        ],
        Ok(EncapsulatedMessage {
            auth_type: AuthType::MD5,
            session_sequence: 1,
            session_id: 2,
            payload: vec![]
        })
    );

    test!(
        truncated_md5,
        [2, 0, 0, 0, 1, 0, 0, 0, 2, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0,],
        Err(UnwrapEncapsulationError::NotEnoughData)
    );
}
