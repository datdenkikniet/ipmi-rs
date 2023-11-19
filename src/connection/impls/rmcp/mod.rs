// LE = least significant byte first = IPMI
// BE = most significant byte first = RMCP/ASF

use std::{
    io::{Error, ErrorKind},
    iter::FusedIterator,
    net::{ToSocketAddrs, UdpSocket},
    num::NonZeroU32,
    time::Duration,
};

use crate::{
    app::auth::{
        self, ActivateSession, Channel, GetChannelAuthenticationCapabilities, GetSessionChallenge,
        PrivilegeLevel,
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
    session_id: Option<NonZeroU32>,
    auth_type: crate::app::auth::AuthType,
    password: [u8; 16],
    _supported_interactions: SupportedInteractions,
    request_sequence: u32,
}

pub struct Rmcp<T> {
    inner: UdpSocket,
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
    PasswordTooLong,
    NoSupportedAuthenticationType,
    GetChannelAuthenticationCapabilities(CommandError<()>),
    GetSessionChallenge(CommandError<()>),
    ActivateSession(CommandError<()>),
}

impl From<std::io::Error> for ActivationError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl Rmcp<Inactive> {
    pub fn new<R: ToSocketAddrs + core::fmt::Debug>(
        remote: R,
        timeout: Duration,
    ) -> std::io::Result<Self> {
        let addrs: Vec<_> = remote.to_socket_addrs()?.collect();

        if addrs.len() != 1 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "You must provide exactly 1 remote address.",
            ));
        }

        log::debug!("Binding socket...");
        let socket = UdpSocket::bind("[::]:0")?;
        socket.set_read_timeout(Some(timeout))?;

        log::debug!("Opening connection");
        socket.connect(addrs[0])?;

        Ok(Self {
            inner: socket,
            responder_addr: 0x20,
            requestor_addr: 0x81,
            requestor_lun: LogicalUnit::Zero,
            ipmb_sequence: 0,
            state: Inactive,
        })
    }

    pub fn activate(
        self,
        username: Option<&str>,
        password: &[u8],
    ) -> Result<Rmcp<Active>, ActivationError> {
        let challenge_command = match GetSessionChallenge::new(auth::AuthType::None, username) {
            Some(v) => v,
            None => return Err(ActivationError::UsernameTooLong),
        };

        if password.len() > 16 {
            return Err(ActivationError::PasswordTooLong);
        }

        let ping = RmcpMessage::new(
            0xFF,
            RmcpClass::ASF(ASFMessage {
                message_tag: 0x00,
                message_type: ASFMessageType::Ping,
            }),
        );

        log::debug!("Starting RMCP activation sequence");
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

        let privilege_level = PrivilegeLevel::Administrator;

        let mut password_padded = [0u8; 16];
        password_padded[..password.len()].copy_from_slice(password);

        let activated = self.convert(Active {
            auth_type: auth::AuthType::None,
            password: password_padded,
            _supported_interactions: supported_interactions,
            session_id: None,
            request_sequence: 0,
        });

        let mut ipmi = crate::Ipmi::new(activated);

        log::debug!("Obtaining channel authentication capabilitiles");

        let authentication_caps = match ipmi.send_recv(GetChannelAuthenticationCapabilities::new(
            Channel::Current,
            privilege_level,
        )) {
            Ok(v) => v,
            Err(e) => return Err(ActivationError::GetChannelAuthenticationCapabilities(e)),
        };

        log::trace!("Authentication capabilities: {:?}", authentication_caps);

        log::debug!("Requesting challenge");

        let challenge = match ipmi.send_recv(challenge_command) {
            Ok(v) => v,
            Err(e) => return Err(ActivationError::GetSessionChallenge(e)),
        };

        let activation_auth_type = authentication_caps
            .best_auth()
            .ok_or(ActivationError::NoSupportedAuthenticationType)?;

        let activate_session: ActivateSession = ActivateSession {
            auth_type: activation_auth_type,
            maxiumum_privilege_level: privilege_level,
            challenge_string: challenge.challenge_string,
            initial_sequence_number: 0xDEAD_BEEF,
        };

        ipmi.inner_mut().state.session_id = Some(challenge.temporary_session_id);
        ipmi.inner_mut().state.auth_type = activation_auth_type;

        log::debug!("Activating session");

        let activation_info = match ipmi.send_recv(activate_session.clone()) {
            Ok(v) => v,
            Err(e) => return Err(ActivationError::ActivateSession(e)),
        };

        log::debug!("Succesfully started a session ({:?})", activation_info);

        let mut me = ipmi.release();

        me.state.request_sequence = activation_info.initial_sequence_number;
        me.state.session_id = Some(activation_info.session_id);

        // TODO: assert the correct thing here
        assert_eq!(activate_session.auth_type, activation_auth_type);

        Ok(me)
    }
}

impl IpmiConnection for Rmcp<Active> {
    type SendError = Error;

    type RecvError = Error;

    type Error = Error;

    fn send(&mut self, request: &mut crate::connection::Request) -> Result<(), Self::SendError> {
        log::trace!("Sending message with auth type {:?}", self.state.auth_type);

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

        let session_sequence = self.state.request_sequence;

        // Only increment the request sequence once a session has been established
        // succesfully.
        if self.state.session_id.is_some() {
            self.state.request_sequence = self.state.request_sequence.wrapping_add(1);
        }

        let auth_type = AuthType::calculate(
            self.state.auth_type,
            &self.state.password,
            self.state.session_id,
            session_sequence,
            &final_data,
        );

        let message = RmcpMessage::new(
            0xFF,
            RmcpClass::IPMI(EncapsulatedMessage {
                auth_type,
                session_sequence,
                session_id: self.state.session_id.map(|v| v.get()).unwrap_or(0),
                payload: final_data,
            }),
        );

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
