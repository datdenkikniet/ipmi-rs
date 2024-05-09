use std::{net::UdpSocket, num::NonZeroU32};

use crate::{
    app::auth::{
        ActivateSession, AuthError, AuthType, ChannelAuthenticationCapabilities,
        GetSessionChallenge, PrivilegeLevel,
    },
    connection::{IpmiConnection, ParseResponseError, Request, Response},
    Ipmi, IpmiError,
};

use super::{
    internal::{validate_ipmb_checksums, IpmbState},
    socket::RmcpIpmiSocket,
    RmcpIpmiError, RmcpIpmiReceiveError, RmcpIpmiSendError,
};

pub use message::Message;

mod auth;
mod md2;
mod message;

#[derive(Debug)]
pub enum ActivationError {
    Io(std::io::Error),
    PasswordTooLong,
    UsernameTooLong,
    GetSessionChallenge(IpmiError<RmcpIpmiError, ParseResponseError<AuthError>>),
    NoSupportedAuthenticationType,
    ActivateSession(IpmiError<RmcpIpmiError, ParseResponseError<AuthError>>),
}

#[derive(Debug)]
pub enum WriteError {
    Io(std::io::Error),
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

pub struct State {
    socket: RmcpIpmiSocket,
    ipmb_state: IpmbState,
    session_id: Option<NonZeroU32>,
    auth_type: crate::app::auth::AuthType,
    password: Option<[u8; 16]>,
    session_sequence: u32,
}

impl core::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("State")
            .field("socket", &self.socket)
            .field("ipmb_state", &self.ipmb_state)
            .field("session_id", &self.session_id)
            .field("auth_type", &self.auth_type)
            .field("password", &"<redacted>")
            .field("session_sequence", &self.session_sequence)
            .finish()
    }
}

impl State {
    pub fn new(socket: UdpSocket) -> Self {
        Self {
            socket: RmcpIpmiSocket::new(socket),
            ipmb_state: Default::default(),
            auth_type: AuthType::None,
            password: None,
            session_id: None,
            session_sequence: 0,
        }
    }

    pub fn release_socket(self) -> UdpSocket {
        self.socket.release()
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

        self.session_sequence = activation_info.initial_sequence_number;
        self.session_id = Some(activation_info.session_id);

        assert_eq!(activate_session.auth_type, activation_auth_type);

        Ok(self)
    }
}

impl IpmiConnection for State {
    type SendError = RmcpIpmiSendError;

    type RecvError = RmcpIpmiReceiveError;

    type Error = RmcpIpmiError;

    fn send(&mut self, request: &mut Request) -> Result<(), RmcpIpmiSendError> {
        log::trace!("Sending message with auth type {:?}", self.auth_type);

        let request_sequence = &mut self.session_sequence;

        // Only increment the request sequence once a session has been established
        // succesfully.
        if self.session_id.is_some() {
            *request_sequence = request_sequence.wrapping_add(1);
        }

        let final_data = super::internal::next_ipmb_message(request, &mut self.ipmb_state);

        let message = Message {
            auth_type: self.auth_type,
            session_sequence_number: self.session_sequence,
            session_id: self.session_id.map(|v| v.get()).unwrap_or(0),
            payload: final_data,
        };

        enum Send {
            Ipmi(WriteError),
            Io(std::io::Error),
        }

        impl From<std::io::Error> for Send {
            fn from(value: std::io::Error) -> Self {
                Self::Io(value)
            }
        }

        match self.socket.send(|buffer| {
            message
                .write_data(self.password.as_ref(), buffer)
                .map_err(Send::Ipmi)
        }) {
            Ok(_) => Ok(()),
            Err(Send::Ipmi(ipmi)) => Err(RmcpIpmiSendError::V1_5(ipmi)),
            Err(Send::Io(io)) => Err(RmcpIpmiSendError::V1_5(WriteError::Io(io))),
        }
    }

    fn recv(&mut self) -> Result<Response, RmcpIpmiReceiveError> {
        let data = self.socket.recv()?;

        let data = Message::from_data(self.password.as_ref(), data)
            .map_err(|e| RmcpIpmiReceiveError::Session(super::UnwrapSessionError::V1_5(e)))?
            .payload;

        if data.len() < 7 {
            return Err(RmcpIpmiReceiveError::NotEnoughData);
        }

        let _req_addr = data[0];
        let netfn = data[1] >> 2;
        let _checksum1 = data[2];
        let _rs_addr = data[3];
        let _rqseq = data[4];
        let cmd = data[5];
        let response_data: Vec<_> = data[6..data.len() - 1].to_vec();
        let _checksum2 = data[data.len() - 1];

        if !validate_ipmb_checksums(&data) {
            return Err(RmcpIpmiReceiveError::IpmbChecksumFailed);
        }

        // TODO: validate sequence number

        if let Some(resp) = Response::new(
            crate::connection::Message::new_raw(netfn, cmd, response_data),
            0,
        ) {
            Ok(resp)
        } else {
            // TODO: need better message here :)
            Err(RmcpIpmiReceiveError::EmptyMessage)
        }
    }

    fn send_recv(&mut self, request: &mut Request) -> Result<Response, Self::Error> {
        self.send(request)?;
        let response = self.recv()?;
        Ok(response)
    }
}
