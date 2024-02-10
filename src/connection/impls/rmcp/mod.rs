// LE = least significant byte first = IPMI
// BE = most significant byte first = RMCP/ASF

use std::{
    io::{Error, ErrorKind},
    net::{ToSocketAddrs, UdpSocket},
    num::NonZeroU32,
    time::Duration,
};

use crate::{
    app::auth::{
        self, ActivateSession, AuthError, GetChannelAuthenticationCapabilities,
        GetSessionChallenge, PrivilegeLevel,
    },
    connection::{Channel, IpmiConnection, LogicalUnit, Response},
    IpmiCommandError,
};

mod wire;

mod protocol;
use protocol::*;

mod encapsulation;

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
    GetSessionChallenge(CommandError<AuthError>),
    ActivateSession(CommandError<AuthError>),
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
            RmcpClass::Asf(ASFMessage {
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
                RmcpClass::Asf(ASFMessage {
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
        wire::send(
            &mut self.inner,
            self.state.auth_type,
            self.requestor_addr,
            self.responder_addr,
            &mut self.ipmb_sequence,
            self.requestor_lun,
            &mut self.state.request_sequence,
            self.state.session_id,
            &self.state.password,
            request,
        )
        .map(|_| ())
    }

    fn recv(&mut self) -> Result<Response, Self::RecvError> {
        wire::recv(&mut self.inner)
    }

    fn send_recv(
        &mut self,
        request: &mut crate::connection::Request,
    ) -> Result<Response, Self::Error> {
        self.send(request)?;
        self.recv()
    }
}
