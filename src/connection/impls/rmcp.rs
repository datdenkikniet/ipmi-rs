use std::{
    io::ErrorKind,
    net::{ToSocketAddrs, UdpSocket},
};

use crate::connection::IpmiConnection;

#[derive(Clone, Debug, PartialEq)]

pub enum ASFMessageType {
    Ping,
    Pong {
        enterprise_number: u32,
        oem_data: u32,
        supported_entities: u8,
        supported_interactions: u8,
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

        println!("{:02X}, {}", type_byte, data_len);

        let data = match type_byte {
            0x80 if data_len == 0 => Self::Ping,
            0x40 if data_len == 0x10 => {
                let enterprise_number = u32::from_le_bytes(data[1..5].try_into().unwrap());
                let oem_data = u32::from_le_bytes(data[5..9].try_into().unwrap());
                let supported_entities = data[9];
                let supported_interactions = data[10];

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
                buffer.extend_from_slice(&[*supported_entities, *supported_interactions]);
                buffer.extend(std::iter::repeat(0).take(6));
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]

pub struct ASFMessage {
    message_tag: u8,
    message_type: ASFMessageType,
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
    IPMI,
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
            RmcpClass::IPMI => todo!(),
            // TODO: OEMDefined data
            RmcpClass::OEMDefined => todo!(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct RmcpMessage {
    version: u8,
    sequence_number: u8,
    class_and_contents: RmcpClass,
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
            RmcpClass::IPMI => 0x07,
            RmcpClass::OEMDefined => 0x08,
        };

        let mut bytes = vec![self.version, 0, self.sequence_number, class];

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

        let class = match class {
            0x06 => RmcpClass::ASF(ASFMessage::from_bytes(&data[4..])?),
            0x07 => RmcpClass::IPMI,
            0x08 => RmcpClass::OEMDefined,
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

pub struct Rmcp {
    inner: UdpSocket,
}

impl Rmcp {
    pub fn new<R: ToSocketAddrs>(remote: R) -> std::io::Result<Self> {
        let addrs: Vec<_> = remote.to_socket_addrs()?.collect();

        if addrs.len() != 1 {
            return Err(std::io::Error::new(
                ErrorKind::InvalidInput,
                "You must provide exactly 1 remote address.",
            ));
        }

        let socket = UdpSocket::bind("[::]:0")?;

        let ping = RmcpMessage::new(
            0xFF,
            RmcpClass::ASF(ASFMessage {
                message_tag: 0x00,
                message_type: ASFMessageType::Ping,
            }),
        );

        socket.connect(addrs[0])?;
        socket.send(&ping.to_bytes())?;

        let mut buf = [0u8; 1024];
        let received = socket.recv(&mut buf)?;

        println!("{:#?}", RmcpMessage::from_bytes(&buf[..received]));

        Ok(Self { inner: socket })
    }
}

impl IpmiConnection for Rmcp {
    type SendError = ();

    type RecvError = ();

    type Error = ();

    fn send(&mut self, request: &mut crate::connection::Request) -> Result<(), Self::SendError> {
        todo!()
    }

    fn recv(&mut self) -> Result<crate::connection::Response, Self::RecvError> {
        todo!()
    }

    fn send_recv(
        &mut self,
        request: &mut crate::connection::Request,
    ) -> Result<crate::connection::Response, Self::Error> {
        todo!()
    }
}

#[test]
fn bruh() {
    let ping = RmcpMessage::new(
        0xFF,
        RmcpClass::ASF(ASFMessage {
            message_tag: 0x00,
            message_type: ASFMessageType::Ping,
        }),
    );

    let data = ping.to_bytes();

    for byte in data {
        print!("\\x{:02X}", byte);
    }

    let data = [
        0x06, 0x00, 0xff, 0x06, 0xbe, 0x11, 0x00, 0x00, 0x40, 0x01, 0x00, 0x10, 0xbe, 0x11, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x81, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    let parsed = RmcpMessage::from_bytes(&data);
    panic!("{:#?}", parsed);
}
