use super::{
    v1_5::Message as V1_5Message,
    v2_0::{self, Message as V2_0Message},
    RmcpIpmiSendError, UnwrapSessionError,
};

#[derive(Clone, Debug)]
pub enum IpmiSessionMessage {
    V1_5(V1_5Message),
    V2_0(V2_0Message),
}

impl IpmiSessionMessage {
    pub fn write_data(
        &self,
        password: Option<&[u8; 16]>,
        buffer: &mut Vec<u8>,
    ) -> Result<(), RmcpIpmiSendError> {
        match self {
            IpmiSessionMessage::V1_5(message) => {
                message.write_data(password, buffer).map_err(Into::into)
            }
            IpmiSessionMessage::V2_0(message) => message
                .write_data(&mut v2_0::CryptoState::default(), buffer)
                .map_err(RmcpIpmiSendError::V2_0),
        }
    }

    pub fn from_data(data: &[u8], password: Option<&[u8; 16]>) -> Result<Self, UnwrapSessionError> {
        if data[0] != 0x06 {
            Ok(Self::V1_5(V1_5Message::from_data(password, data)?))
        } else {
            Ok(Self::V2_0(V2_0Message::from_data(
                &mut v2_0::CryptoState::default(),
                data,
            )?))
        }
    }
}
