#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RmcpClass {
    pub ty: RmcpType,
    pub is_ack: bool,
}

impl From<RmcpClass> for u8 {
    fn from(value: RmcpClass) -> Self {
        let ack_bit = (value.is_ack as u8) << 7;

        let value = match value.ty {
            RmcpType::Asf => 0x06,
            RmcpType::Ipmi => 0x07,
            RmcpType::OemDefined => 0x08,
        };

        value | ack_bit
    }
}

impl TryFrom<u8> for RmcpClass {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let is_ack = (value & 0x80) == 0x80;
        let value = value & 0x7F;

        let ty = match value {
            0x06 => RmcpType::Asf,
            0x07 => RmcpType::Ipmi,
            0x08 => RmcpType::OemDefined,
            _ => return Err(()),
        };

        Ok(Self { is_ack, ty })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RmcpType {
    Asf,
    Ipmi,
    OemDefined,
}

#[derive(Debug)]
pub enum RmcpHeaderError {
    /// There was not enough data in the packet to parse a valid RMCP message.
    NotEnoughData,
    /// The RMCP packet contained an invalid ASF message.
    InvalidASFMessage,
    /// The class of the RMCP packet was not valid.
    InvalidRmcpClass,
}

#[derive(Clone, Debug)]
pub struct RmcpHeader {
    version: u8,
    sequence_number: u8,
    class: RmcpClass,
}

impl RmcpHeader {
    fn new(sequence_number: u8, ty: RmcpType) -> Self {
        Self {
            version: 6,
            sequence_number,
            class: RmcpClass { ty, is_ack: false },
        }
    }

    pub fn class(&self) -> &RmcpClass {
        &self.class
    }

    pub fn new_asf(sequence: u8) -> Self {
        Self::new(sequence, RmcpType::Asf)
    }

    pub fn new_ipmi() -> Self {
        Self::new(0xFF, RmcpType::Ipmi)
    }

    pub fn write_infallible<F>(&self, data: F) -> Vec<u8>
    where
        F: FnOnce(&mut Vec<u8>),
    {
        fn infallible<FInner>(
            data: FInner,
            buffer: &mut Vec<u8>,
        ) -> Result<(), core::convert::Infallible>
        where
            FInner: FnOnce(&mut Vec<u8>),
        {
            data(buffer);
            Ok(())
        }

        self.write(|buffer| infallible(data, buffer)).unwrap()
    }

    pub fn write<F, E>(&self, data: F) -> Result<Vec<u8>, E>
    where
        F: FnOnce(&mut Vec<u8>) -> Result<(), E>,
    {
        let class = u8::from(self.class);

        let sequence_number = if self.class.ty == RmcpType::Ipmi {
            0xFF
        } else {
            self.sequence_number
        };

        let mut bytes = vec![self.version, 0, sequence_number, class];

        data(&mut bytes)?;

        Ok(bytes)
    }

    pub fn from_bytes<'a>(data: &'a [u8]) -> Result<(Self, &'a [u8]), RmcpHeaderError> {
        if data.len() < 4 {
            return Err(RmcpHeaderError::NotEnoughData);
        }

        let version = data[0];
        let sequence_number = data[2];
        let class = data[3];

        let data = &data[4..];

        let class = RmcpClass::try_from(class).map_err(|_| RmcpHeaderError::InvalidRmcpClass)?;

        Ok((
            Self {
                version,
                sequence_number,
                class,
            },
            data,
        ))
    }
}
