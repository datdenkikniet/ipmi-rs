use super::encapsulation::EncapsulatedMessage;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SupportedInteractions {
    pub rcmp_security: bool,
    pub dmtf_dash: bool,
}

impl TryFrom<u8> for SupportedInteractions {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let rcmp_security = (value & 0x80) == 0x80;
        let dmtf_dash = (value & 0x10) == 0x10;

        // All of these bits must be 0
        if (value & 0b01011111) != 0 {
            return Err(());
        }

        Ok(Self {
            dmtf_dash,
            rcmp_security,
        })
    }
}

impl From<SupportedInteractions> for u8 {
    fn from(value: SupportedInteractions) -> Self {
        let security = if value.rcmp_security { 0x80 } else { 0x00 };
        let dmtf_dash = if value.dmtf_dash { 0x10 } else { 0x00 };
        security | dmtf_dash
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SupportedEntities {
    pub ipmi: bool,
}

impl From<u8> for SupportedEntities {
    fn from(value: u8) -> Self {
        let ipmi = (value & 0x80) == 0x80;
        Self { ipmi }
    }
}

impl From<SupportedEntities> for u8 {
    fn from(value: SupportedEntities) -> Self {
        if value.ipmi {
            0x80
        } else {
            0x00
        }
    }
}

#[derive(Clone, Debug, PartialEq)]

pub enum ASFMessageType {
    Ping,
    Pong {
        enterprise_number: u32,
        oem_data: u32,
        supported_entities: SupportedEntities,
        supported_interactions: SupportedInteractions,
    },
}

impl ASFMessageType {
    fn type_byte(&self) -> u8 {
        match self {
            ASFMessageType::Ping => 0x80,
            ASFMessageType::Pong { .. } => 0x40,
        }
    }

    fn from_type_byte_and_data(type_byte: u8, data: &[u8]) -> Option<Self> {
        if data.len() < 1 {
            return None;
        }

        let data_len = data[0];

        let data = match type_byte {
            0x80 if data_len == 0 => Self::Ping,
            0x40 if data_len == 0x10 => {
                let enterprise_number = u32::from_le_bytes(data[1..5].try_into().unwrap());
                let oem_data = u32::from_le_bytes(data[5..9].try_into().unwrap());
                let supported_entities = SupportedEntities::from(data[9]);
                let supported_interactions = SupportedInteractions::try_from(data[10]).ok()?;

                Self::Pong {
                    enterprise_number,
                    oem_data,
                    supported_entities,
                    supported_interactions,
                }
            }
            _ => return None,
        };

        Some(data)
    }

    fn write_data(&self, buffer: &mut Vec<u8>) {
        match self {
            // No data
            ASFMessageType::Ping => {
                // Data length
                buffer.push(0);
            }
            ASFMessageType::Pong {
                enterprise_number,
                oem_data,
                supported_entities,
                supported_interactions,
            } => {
                // Data length
                buffer.push(0x10);
                buffer.extend_from_slice(&enterprise_number.to_le_bytes());
                buffer.extend_from_slice(&oem_data.to_le_bytes());
                buffer.extend_from_slice(&[
                    u8::from(*supported_entities),
                    u8::from(*supported_interactions),
                ]);
                buffer.extend(std::iter::repeat(0).take(6));
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]

pub struct ASFMessage {
    pub message_tag: u8,
    pub message_type: ASFMessageType,
}

impl ASFMessage {
    fn write_data(&self, buffer: &mut Vec<u8>) {
        buffer.extend_from_slice(&4542u32.to_le_bytes());

        buffer.push(self.message_type.type_byte());
        buffer.push(self.message_tag);
        buffer.push(0x00);

        self.message_type.write_data(buffer);
    }

    fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 8 {
            return None;
        }

        if data[..4] != 4542u32.to_le_bytes() {
            return None;
        }

        let message_type = data[4];
        let message_tag = data[5];

        if data[6] != 0 {
            return None;
        }

        let message_type = ASFMessageType::from_type_byte_and_data(message_type, &data[7..])?;

        Some(Self {
            message_tag,
            message_type,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum RmcpClass {
    Ack(u8),
    ASF(ASFMessage),
    IPMI(EncapsulatedMessage),
    OEMDefined,
}

impl RmcpClass {
    fn write_data(&self, buffer: &mut Vec<u8>) {
        match self {
            // No data
            RmcpClass::Ack(_) => {}
            // ASF data
            RmcpClass::ASF(message) => message.write_data(buffer),
            // TODO: IPMI data
            RmcpClass::IPMI(message) => message.write_data(buffer),
            // TODO: OEMDefined data
            RmcpClass::OEMDefined => todo!(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct RmcpMessage {
    pub version: u8,
    pub sequence_number: u8,
    pub class_and_contents: RmcpClass,
}

impl RmcpMessage {
    pub fn new(sequence_number: u8, contents: RmcpClass) -> Self {
        Self {
            version: 6,
            sequence_number,
            class_and_contents: contents,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let class = match self.class_and_contents {
            RmcpClass::Ack(value) => value | 0x80,
            RmcpClass::ASF(_) => 0x06,
            RmcpClass::IPMI(_) => 0x07,
            RmcpClass::OEMDefined => 0x08,
        };

        let sequence_number = if matches!(self.class_and_contents, RmcpClass::IPMI(_)) {
            0xFF
        } else {
            self.sequence_number
        };

        let mut bytes = vec![self.version, 0, sequence_number, class];

        self.class_and_contents.write_data(&mut bytes);

        bytes
    }

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 4 {
            return None;
        }

        let version = data[0];
        let sequence_number = data[2];
        let class = data[3];

        let data = &data[4..];

        let class = match class {
            0x06 => RmcpClass::ASF(ASFMessage::from_bytes(data)?),
            0x07 => RmcpClass::IPMI(EncapsulatedMessage::from_bytes(data)?),
            0x08 => RmcpClass::OEMDefined,
            _ if class & 0x80 == 0x80 => RmcpClass::Ack(class & 0x7F),
            _ => {
                return None;
            }
        };

        Some(Self {
            version,
            sequence_number,
            class_and_contents: class,
        })
    }
}
