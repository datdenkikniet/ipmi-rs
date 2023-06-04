use crate::app::auth;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AuthType {
    None,
    MD2([u8; 16]),
    MD5([u8; 16]),
    Key([u8; 16]),
}

impl AuthType {
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    pub fn auth_code(&self) -> Option<&[u8; 16]> {
        match self {
            AuthType::None => None,
            AuthType::MD2(b) | AuthType::MD5(b) | AuthType::Key(b) => Some(b),
        }
    }

    pub fn calculate(
        auth_type: auth::AuthType,
        password: &[u8; 16],
        session_id: u32,
        session_seq: u32,
        data: &[u8],
    ) -> AuthType {
        match auth_type {
            auth::AuthType::None => Self::None,
            auth::AuthType::MD2 => todo!(),
            auth::AuthType::MD5 => {
                let mut context = md5::Context::new();
                context.consume(password);
                context.consume(&session_id.to_le_bytes());
                context.consume(data);
                context.consume(session_seq.to_le_bytes());
                context.consume(password);

                Self::MD5(context.compute().0)
            }
            auth::AuthType::Key => Self::Key(password.clone()),
        }
    }
}

impl From<AuthType> for u8 {
    fn from(value: AuthType) -> Self {
        match value {
            AuthType::None => 0x00,
            AuthType::MD2(_) => 0x01,
            AuthType::MD5(_) => 0x02,
            AuthType::Key(_) => 0x04,
        }
    }
}

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

#[derive(Debug, Clone, PartialEq)]
pub struct EncapsulatedMessage {
    pub auth_type: AuthType,
    pub session_sequence: u32,
    pub session_id: u32,
    pub payload: Vec<u8>,
}

impl EncapsulatedMessage {
    pub fn write_data(&self, buffer: &mut Vec<u8>) {
        buffer.push(self.auth_type.into());
        buffer.extend_from_slice(&self.session_sequence.to_le_bytes());
        buffer.extend_from_slice(&self.session_id.to_le_bytes());

        if let Some(auth_code) = self.auth_type.auth_code() {
            buffer.extend_from_slice(auth_code);
        }

        buffer.push(self.payload.len() as u8);
        buffer.extend_from_slice(&self.payload);

        // Legacy PAD
        buffer.push(0);
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < 10 {
            return Err("Not enough data");
        }

        let auth_type = match data[0] {
            0x00 => AuthType::None,
            _ => {
                if data.len() < 26 {
                    return Err("Not enough data for authenticated");
                }

                let auth_code = data[9..25].try_into().unwrap();

                match data[0] {
                    0x01 => AuthType::MD2(auth_code),
                    0x02 => AuthType::MD5(auth_code),
                    0x04 => AuthType::Key(auth_code),
                    _ => return Err("Unkonwn auth type"),
                }
            }
        };

        let session_sequence = u32::from_le_bytes(data[1..5].try_into().unwrap());
        let session_id = u32::from_le_bytes(data[5..9].try_into().unwrap());

        let data = if auth_type.is_none() {
            &data[9..]
        } else {
            &data[25..]
        };

        let data_len = data[0];
        let data = &data[1..];

        let payload = if data_len == 0 && data.len() == 0 {
            Vec::new()
        } else if data.len() == data_len as usize {
            data.iter().map(|v| *v).collect()
        } else {
            return Err("Payload len is not correct");
        };

        Ok(Self {
            auth_type,
            session_sequence,
            session_id,
            payload,
        })
    }

    pub fn _verify(&self, _checksum: [u8; 16]) -> bool {
        todo!()
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

                let encapsulated = EncapsulatedMessage::from_bytes(&data);

                assert_eq!(encapsulated, $then);
            }
        };
    }

    test!(
        empty_noauth,
        [0, 0, 0, 0, 1, 0, 0, 0, 2, 0],
        Ok(EncapsulatedMessage {
            auth_type: AuthType::None,
            session_sequence: 1,
            session_id: 2,
            payload: vec![]
        })
    );

    test!(
        nonempty_noauth,
        [0, 0, 0, 0, 1, 0, 0, 0, 2, 5, 1, 2, 3, 4, 5],
        Ok(EncapsulatedMessage {
            auth_type: AuthType::None,
            session_sequence: 1,
            session_id: 2,
            payload: vec![1, 2, 3, 4, 5]
        })
    );

    test!(
        nonempty_incorrect_len,
        [0, 0, 0, 0, 1, 0, 0, 0, 2, 5, 1, 2, 3, 4],
        Err("Not enough data")
    );

    test!(
        empty_md5,
        [2, 0, 0, 0, 1, 0, 0, 0, 2, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 0],
        Ok(EncapsulatedMessage {
            auth_type: AuthType::MD5([1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0]),
            session_sequence: 1,
            session_id: 2,
            payload: vec![]
        })
    );

    test!(
        truncated_md5,
        [2, 0, 0, 0, 1, 0, 0, 0, 2, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0,],
        Err("Not enough data for authenticated")
    );
}
