use super::{
    v1_5::{Message as V1_5Message, ReadError as V1_5ReadError, WriteError as V1_5WriteError},
    v2_0::Message as V2_0Message,
    ASFMessage,
};

#[derive(Debug, Clone, PartialEq)]
pub enum WriteError {
    V1_5(V1_5WriteError),
    V2_0(&'static str),
}

impl From<V1_5WriteError> for WriteError {
    fn from(value: V1_5WriteError) -> Self {
        Self::V1_5(value)
    }
}

impl From<&'static str> for WriteError {
    fn from(value: &'static str) -> Self {
        Self::V2_0(value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReadError {
    V1_5(V1_5ReadError),
    V2_0(&'static str),
}

impl From<V1_5ReadError> for ReadError {
    fn from(value: V1_5ReadError) -> Self {
        Self::V1_5(value)
    }
}

impl From<&'static str> for ReadError {
    fn from(value: &'static str) -> Self {
        Self::V2_0(value)
    }
}

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
    ) -> Result<(), WriteError> {
        match self {
            IpmiSessionMessage::V1_5(message) => {
                message.write_data(password, buffer).map_err(Into::into)
            }
            IpmiSessionMessage::V2_0(message) => message
                .write_data(&mut super::v2_0::CryptoState::default(), buffer)
                .map_err(Into::into),
        }
    }

    pub fn from_data(data: &[u8], password: Option<&[u8; 16]>) -> Result<Self, ReadError> {
        if data[0] != 0x06 {
            Ok(Self::V1_5(V1_5Message::from_data(password, data)?))
        } else {
            Ok(Self::V2_0(V2_0Message::from_data(
                &mut super::v2_0::CryptoState::default(),
                data,
            )?))
        }
    }
}

#[derive(Clone, Debug)]
pub enum RmcpClass {
    Ack(u8),
    Asf(ASFMessage),
    Ipmi(IpmiSessionMessage),
    OemDefined,
}

impl RmcpClass {
    fn write_data(
        &self,
        password: Option<&[u8; 16]>,
        buffer: &mut Vec<u8>,
    ) -> Result<(), WriteError> {
        match self {
            RmcpClass::Ack(_) => {
                log::trace!("Received RMCP ACK, but do not know how to validate sequence number.");
                Ok(())
            }
            RmcpClass::Asf(message) => {
                message.write_data(buffer);
                Ok(())
            }
            RmcpClass::Ipmi(message) => message.write_data(password, buffer),
            // TODO: OEMDefined data
            RmcpClass::OemDefined => todo!(),
        }
    }
}

#[derive(Debug)]
pub enum RmcpUnwrapError {
    /// There was not enough data in the packet to parse a valid RMCP message.
    NotEnoughData,
    /// An error occurred while trying to unwrap the encapsulated RMCP
    /// message.
    UnwrapEncapsulation(ReadError),
    /// The RMCP packet contained an invalid ASF message.
    InvalidASFMessage,
    /// The class of the RMCP packet was not valid.
    InvalidRmcpClass,
}

impl From<ReadError> for RmcpUnwrapError {
    fn from(value: ReadError) -> Self {
        Self::UnwrapEncapsulation(value)
    }
}

#[derive(Clone, Debug)]
pub struct RmcpMessage {
    pub version: u8,
    pub sequence_number: u8,
    pub class_and_contents: RmcpClass,
}

impl From<IpmiSessionMessage> for RmcpMessage {
    fn from(value: IpmiSessionMessage) -> Self {
        Self::new(0xFF, RmcpClass::Ipmi(value))
    }
}

impl RmcpMessage {
    pub fn new(sequence_number: u8, contents: RmcpClass) -> Self {
        Self {
            version: 6,
            sequence_number,
            class_and_contents: contents,
        }
    }

    pub fn to_bytes(&self, password: Option<&[u8; 16]>) -> Result<Vec<u8>, WriteError> {
        let class = match self.class_and_contents {
            RmcpClass::Ack(value) => value | 0x80,
            RmcpClass::Asf(_) => 0x06,
            RmcpClass::Ipmi(_) => 0x07,
            RmcpClass::OemDefined => 0x08,
        };

        let sequence_number = if matches!(self.class_and_contents, RmcpClass::Ipmi(_)) {
            0xFF
        } else {
            self.sequence_number
        };

        let mut bytes = vec![self.version, 0, sequence_number, class];

        self.class_and_contents.write_data(password, &mut bytes)?;

        Ok(bytes)
    }

    pub fn from_bytes(password: Option<&[u8; 16]>, data: &[u8]) -> Result<Self, RmcpUnwrapError> {
        if data.len() < 4 {
            return Err(RmcpUnwrapError::NotEnoughData);
        }

        let version = data[0];
        let sequence_number = data[2];
        let class = data[3];

        let data = &data[4..];

        let class = match class {
            0x06 => RmcpClass::Asf(
                ASFMessage::from_bytes(data).ok_or(RmcpUnwrapError::InvalidASFMessage)?,
            ),
            0x07 => RmcpClass::Ipmi(IpmiSessionMessage::from_data(data, password)?),
            0x08 => RmcpClass::OemDefined,
            _ if class & 0x80 == 0x80 => RmcpClass::Ack(class & 0x7F),
            _ => {
                return Err(RmcpUnwrapError::InvalidRmcpClass);
            }
        };

        Ok(Self {
            version,
            sequence_number,
            class_and_contents: class,
        })
    }
}
