use std::{net::UdpSocket, num::NonZeroU32};

use crate::{
    app::auth::{
        ActivateSession, AuthError, AuthType, ChannelAuthenticationCapabilities,
        GetSessionChallenge, PrivilegeLevel,
    },
    connection::{IpmiConnection, ParseResponseError, Request, Response},
    Ipmi, IpmiError,
};

use super::{internal::IpmbState, RmcpError};

pub use message::Message;

mod auth;
mod md2;
mod message;
mod wire;

#[derive(Debug)]
pub enum ActivationError {
    PasswordTooLong,
    UsernameTooLong,
    GetSessionChallenge(IpmiError<RmcpError, ParseResponseError<AuthError>>),
    NoSupportedAuthenticationType,
    ActivateSession(IpmiError<RmcpError, ParseResponseError<AuthError>>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WriteError {
    /// A request was made to calculate the auth code for a message authenticated
    /// using a method that requires a password, but no password was provided.
    MissingPassword,
    /// The payload length of for the V1_5 packet is larger than the maximum
    /// allowed size (256 bytes).
    PayloadTooLarge(usize),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReadError {
    /// There is not enough data in the packet to form a valid [`EncapsulatedMessage`].
    NotEnoughData,
    /// The auth type provided is not supported.
    UnsupportedAuthType(u8),
    /// There is a mismatch between the payload length field and the
    /// actual length of the payload.
    IncorrectPayloadLen,
    /// The auth code of the message is not correct.
    AuthcodeError,
}

// TODO: override debug to avoid printing password
#[derive(Debug)]
pub struct State {
    ipbm_state: IpmbState,
    socket: UdpSocket,
    session_id: Option<NonZeroU32>,
    auth_type: crate::app::auth::AuthType,
    password: Option<[u8; 16]>,
    request_sequence: u32,
}

impl State {
    pub fn new(socket: UdpSocket) -> Self {
        Self {
            socket,
            ipbm_state: Default::default(),
            auth_type: AuthType::None,
            password: None,
            session_id: None,
            request_sequence: 0,
        }
    }

    pub fn activate(
        mut self,
        authentication_caps: &ChannelAuthenticationCapabilities,
        privilege_level: PrivilegeLevel,
        username: Option<&str>,
        password: Option<&[u8]>,
    ) -> Result<Self, ActivationError> {
        let password = if let Some(password) = password {
            if password.len() > 16 {
                return Err(ActivationError::PasswordTooLong);
            } else {
                let mut padded = [0u8; 16];
                padded[..password.len()].copy_from_slice(password);
                Some(padded)
            }
        } else {
            None
        };

        self.password = password;

        let mut ipmi = Ipmi::new(self);

        log::debug!("Requesting challenge");

        let challenge_command = match GetSessionChallenge::new(AuthType::None, username) {
            Some(v) => v,
            None => return Err(ActivationError::UsernameTooLong),
        };

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

        ipmi.inner_mut().session_id = Some(challenge.temporary_session_id);
        ipmi.inner_mut().auth_type = activation_auth_type;

        log::debug!("Activating session");

        let activation_info = match ipmi.send_recv(activate_session.clone()) {
            Ok(v) => v,
            Err(e) => return Err(ActivationError::ActivateSession(e)),
        };

        log::debug!("Succesfully started a session ({:?})", activation_info);

        self = ipmi.release();

        self.request_sequence = activation_info.initial_sequence_number;
        self.session_id = Some(activation_info.session_id);

        // TODO: assert the correct thing here
        assert_eq!(activate_session.auth_type, activation_auth_type);

        Ok(self)
    }
}

impl IpmiConnection for State {
    type SendError = RmcpError;

    type RecvError = RmcpError;

    type Error = RmcpError;

    fn send(&mut self, request: &mut Request) -> Result<(), RmcpError> {
        wire::send_v1_5(
            &mut self.socket,
            self.auth_type,
            self.ipbm_state.requestor_addr,
            self.ipbm_state.responder_addr,
            &mut self.ipbm_state.ipmb_sequence,
            self.ipbm_state.requestor_lun,
            &mut self.request_sequence,
            self.session_id,
            self.password.as_ref(),
            request,
        )
        .map(|_| ())
    }

    fn recv(&mut self) -> Result<Response, RmcpError> {
        wire::recv(self.password.as_ref(), &mut self.socket)
    }

    fn send_recv(&mut self, request: &mut Request) -> Result<Response, Self::Error> {
        self.send(request)?;
        self.recv()
    }
}
