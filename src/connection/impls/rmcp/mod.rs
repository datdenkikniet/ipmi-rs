#![allow(unused)]

use std::{
    io::ErrorKind,
    net::{ToSocketAddrs, UdpSocket},
    time::Duration,
};

use crate::connection::IpmiConnection;

mod rmcp;
use rmcp::*;

mod encapsulation;
use encapsulation::*;

pub struct Rmcp {
    inner: UdpSocket,
    supported_interactions: SupportedInteractions,
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
        socket.set_read_timeout(Some(Duration::from_secs(2)))?;

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

        let pong = RmcpMessage::from_bytes(&buf[..received]);

        println!("{pong:#?}");

        let (supported_entities, supported_interactions) = if let Some(RmcpMessage {
            class_and_contents:
                RmcpClass::ASF(ASFMessage {
                    message_type:
                        ASFMessageType::Pong {
                            supported_entities,
                            supported_interactions,
                            ..
                        },
                    ..
                }),
            ..
        }) = pong
        {
            (supported_entities, supported_interactions)
        } else {
            return Err(std::io::Error::new(
                ErrorKind::Other,
                "Invalid response from remote",
            ));
        };

        if !supported_entities.ipmi {
            return Err(std::io::Error::new(
                ErrorKind::Unsupported,
                "Remote does not support IPMI entity.",
            ));
        }

        Ok(Self {
            inner: socket,
            supported_interactions,
        })
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
