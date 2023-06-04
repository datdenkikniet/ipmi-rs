use std::{
    io::{Error, ErrorKind},
    iter::FusedIterator,
    net::{ToSocketAddrs, UdpSocket},
    time::Duration,
};

use crate::{
    app::auth::{
        self, Channel, GetChannelAuthenticationCapabilities, GetSessionChallenge, PrivilegeLevel,
    },
    connection::{IpmiConnection, LogicalUnit, Message, Response},
    IpmiCommandError,
};

mod rmcp;
use rmcp::*;

mod encapsulation;
use encapsulation::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Inactive;

pub struct Active {
    supported_interactions: SupportedInteractions,
}

pub struct Rmcp<T> {
    inner: UdpSocket,
    request_sequence: u32,
    ipmb_sequence: u8,
    responder_addr: u8,
    requestor_addr: u8,
    requestor_lun: LogicalUnit,
    state: T,
}

impl<T> Rmcp<T> {
    fn checksum(data: impl IntoIterator<Item = u8>) -> impl Iterator<Item = u8> {
        struct ChecksumIterator<I> {
            checksum: u8,
            yielded_checksum: bool,
            inner: I,
        }

        impl<I: Iterator<Item = u8>> Iterator for ChecksumIterator<I> {
            type Item = u8;

            fn next(&mut self) -> Option<Self::Item> {
                if let Some(value) = self.inner.next() {
                    self.checksum = self.checksum.wrapping_add(value);
                    Some(value)
                } else if !self.yielded_checksum {
                    self.yielded_checksum = true;
                    self.checksum = !self.checksum;
                    self.checksum = self.checksum.wrapping_add(1);
                    Some(self.checksum)
                } else {
                    None
                }
            }
        }

        impl<I: Iterator<Item = u8>> FusedIterator for ChecksumIterator<I> {}

        ChecksumIterator {
            checksum: 0,
            yielded_checksum: false,
            inner: data.into_iter(),
        }
    }

    fn convert<O>(self, new_state: O) -> Rmcp<O> {
        Rmcp {
            inner: self.inner,
            request_sequence: self.request_sequence,
            ipmb_sequence: self.ipmb_sequence,
            responder_addr: self.responder_addr,
            requestor_addr: self.requestor_addr,
            requestor_lun: self.requestor_lun,
            state: new_state,
        }
    }
}

type CommandError<T> = IpmiCommandError<<Rmcp<Active> as IpmiConnection>::Error, T>;

#[derive(Debug)]
pub enum ActivationError {
    Io(Error),
    UsernameTooLong,
    GetChannelAuthenticationCapabilities(CommandError<()>),
    GetSessionChallenge(CommandError<()>),
}

impl From<std::io::Error> for ActivationError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl Rmcp<Inactive> {
    pub fn new<R: ToSocketAddrs>(remote: R) -> std::io::Result<Self> {
        let addrs: Vec<_> = remote.to_socket_addrs()?.collect();

        if addrs.len() != 1 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "You must provide exactly 1 remote address.",
            ));
        }

        let socket = UdpSocket::bind("[::]:0")?;
        socket.set_read_timeout(Some(Duration::from_secs(2)))?;
        socket.connect(addrs[0])?;

        Ok(Self {
            inner: socket,
            request_sequence: 0,
            responder_addr: 0x20,
            requestor_addr: 0x81,
            requestor_lun: LogicalUnit::Zero,
            ipmb_sequence: 0,
            state: Inactive,
        })
    }

    pub fn activate(self, username: Option<&str>) -> Result<Rmcp<Active>, ActivationError> {
        let challenge_command = match GetSessionChallenge::new(auth::AuthType::None, username) {
            Some(v) => v,
            None => return Err(ActivationError::UsernameTooLong),
        };

        let ping = RmcpMessage::new(
            0xFF,
            RmcpClass::ASF(ASFMessage {
                message_tag: 0x00,
                message_type: ASFMessageType::Ping,
            }),
        );

        self.inner.send(&ping.to_bytes())?;

        let mut buf = [0u8; 1024];
        let received = self.inner.recv(&mut buf)?;

        let pong = RmcpMessage::from_bytes(&buf[..received]);

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
            return Err(Error::new(ErrorKind::Other, "Invalid response from remote").into());
        };

        if !supported_entities.ipmi {
            return Err(Error::new(
                ErrorKind::Unsupported,
                "Remote does not support IPMI entity.",
            )
            .into());
        }

        let activated = self.convert(Active {
            supported_interactions,
        });

        let mut ipmi = crate::Ipmi::new(activated);

        let output = match ipmi.send_recv(GetChannelAuthenticationCapabilities::new(
            Channel::Current,
            PrivilegeLevel::Administrator,
        )) {
            Ok(v) => v,
            Err(e) => return Err(ActivationError::GetChannelAuthenticationCapabilities(e)),
        };

        // assert!(output.none);

        let challenge = match ipmi.send_recv(challenge_command) {
            Ok(v) => v,
            Err(e) => return Err(ActivationError::GetSessionChallenge(e)),
        };

        panic!("{challenge:?}");
    }
}

impl IpmiConnection for Rmcp<Active> {
    type SendError = Error;

    type RecvError = Error;

    type Error = Error;

    fn send(&mut self, request: &mut crate::connection::Request) -> Result<(), Self::SendError> {
        let rs_addr = self.responder_addr;
        let netfn_rslun: u8 = (request.netfn().request_value() << 2) | request.lun().value();

        let first_part = Self::checksum([rs_addr, netfn_rslun]);

        let req_addr = self.requestor_addr;

        let ipmb_sequence = self.ipmb_sequence;
        self.ipmb_sequence = self.ipmb_sequence.wrapping_add(1);

        let reqseq_lun = (ipmb_sequence << 2) | self.requestor_lun.value();
        let cmd = request.cmd();
        let second_part = Self::checksum(
            [req_addr, reqseq_lun, cmd]
                .into_iter()
                .chain(request.data().iter().map(|v| *v)),
        );

        let final_data: Vec<_> = first_part.chain(second_part).collect();

        let message = RmcpMessage::new(
            0xFF,
            RmcpClass::IPMI(EncapsulatedMessage {
                auth_type: AuthType::None,
                session_sequence: self.request_sequence,
                session_id: 0,
                payload: final_data,
            }),
        );

        self.request_sequence = self.request_sequence.wrapping_add(1);

        self.inner.send(&message.to_bytes())?;
        Ok(())
    }

    fn recv(&mut self) -> Result<Response, Self::RecvError> {
        let mut buffer = [0u8; 1024];
        let received_bytes = self.inner.recv(&mut buffer)?;

        if received_bytes < 8 {
            return Err(Error::new(ErrorKind::Other, "Incomplete response"));
        }

        let data = &buffer[..received_bytes];

        let rcmp_message = RmcpMessage::from_bytes(data)
            .ok_or(Error::new(ErrorKind::Other, "RMCP response not recognized"))?;

        let encapsulated_message = if let RmcpMessage {
            class_and_contents: RmcpClass::IPMI(message),
            ..
        } = rcmp_message
        {
            message
        } else {
            return Err(Error::new(
                ErrorKind::Other,
                "RMCP response does not have IPMI class",
            ));
        };

        let data = encapsulated_message.payload;

        let _req_addr = data[0];
        let netfn = data[1] >> 2;
        let _checksum1 = data[2];
        let _rs_addr = data[3];
        let _rqseq = data[4];
        let cmd = data[5];
        let response_data: Vec<_> = data[6..data.len() - 1].iter().map(|v| *v).collect();
        let _checksum2 = data[data.len() - 1];

        let response =
            if let Some(resp) = Response::new(Message::new_raw(netfn, cmd, response_data), 0) {
                resp
            } else {
                return Err(Error::new(ErrorKind::Other, "Response data was empty"));
            };

        Ok(response)
    }

    fn send_recv(
        &mut self,
        request: &mut crate::connection::Request,
    ) -> Result<Response, Self::Error> {
        self.send(request)?;
        self.recv()
    }
}

#[test]
pub fn checksum() {
    let output: Vec<_> = Rmcp::<Inactive>::checksum([0x20, 0x06 << 2]).collect();
    panic!("{:02X?}", output);
}
