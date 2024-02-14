use std::{net::UdpSocket, num::NonZeroU32};

use crate::{
    app::auth::{
        ActivateSession, AuthError, AuthType, ChannelAuthenticationCapabilities,
        GetSessionChallenge, PrivilegeLevel,
    },
    connection::{rmcp::IpmiSessionMessage, IpmiConnection, ParseResponseError, Request, Response},
    Ipmi, IpmiError,
};

use super::{
    internal::IpmbState, socket::RmcpIpmiSocket, RmcpIpmiError, RmcpIpmiReceiveError,
    RmcpIpmiSendError,
};

pub use message::Message;

mod auth;
mod checksum;
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

// TODO: override debug to avoid printing password
#[derive(Debug)]
pub struct State {
    socket: RmcpIpmiSocket,
    ipbm_state: IpmbState,
    session_id: Option<NonZeroU32>,
    auth_type: crate::app::auth::AuthType,
    password: Option<[u8; 16]>,
    session_sequence: u32,
}

impl State {
    pub fn new(socket: UdpSocket) -> Self {
        Self {
            socket: RmcpIpmiSocket::new(socket),
            ipbm_state: Default::default(),
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

        // TODO: assert the correct thing here
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

        let IpmbState {
            ipmb_sequence,
            responder_addr: rs_addr,
            requestor_addr,
            requestor_lun,
        } = &mut self.ipbm_state;

        let netfn_rslun: u8 =
            (request.netfn().request_value() << 2) | request.target().lun().value();

        let first_part = checksum::checksum([*rs_addr, netfn_rslun]);

        let req_addr = *requestor_addr;

        let ipmb_sequence_val = *ipmb_sequence;
        *ipmb_sequence = ipmb_sequence.wrapping_add(1);

        let reqseq_lun = (ipmb_sequence_val << 2) | requestor_lun.value();
        let cmd = request.cmd();
        let second_part = checksum::checksum(
            [req_addr, reqseq_lun, cmd]
                .into_iter()
                .chain(request.data().iter().copied()),
        );

        let final_data: Vec<_> = first_part.chain(second_part).collect();

        let request_sequence = &mut self.session_sequence;

        // Only increment the request sequence once a session has been established
        // succesfully.
        if self.session_id.is_some() {
            *request_sequence = request_sequence.wrapping_add(1);
        }

        let message = IpmiSessionMessage::V1_5(Message {
            auth_type: self.auth_type,
            session_sequence_number: self.session_sequence,
            session_id: self.session_id.map(|v| v.get()).unwrap_or(0),
            payload: final_data,
        });

        enum Send {
            Ipmi(RmcpIpmiSendError),
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
            Err(Send::Ipmi(ipmi)) => Err(ipmi),
            Err(Send::Io(io)) => Err(RmcpIpmiSendError::V1_5(WriteError::Io(io))),
        }
    }

    fn recv(&mut self) -> Result<Response, RmcpIpmiReceiveError> {
        let data = self.socket.recv()?;

        let encapsulated_message = IpmiSessionMessage::from_data(data, self.password.as_ref())
            .map_err(RmcpIpmiReceiveError::Session)?;

        let data = match encapsulated_message {
            IpmiSessionMessage::V1_5(Message { payload, .. }) => payload,
            IpmiSessionMessage::V2_0 { .. } => {
                panic!("Received IPMI V2.0 message in V1.5 session.")
            }
        };

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

        // TODO: validate sequence, checksums, etc.

        let response = if let Some(resp) = Response::new(
            crate::connection::Message::new_raw(netfn, cmd, response_data),
            0,
        ) {
            resp
        } else {
            // TODO: need better message here :)
            return Err(RmcpIpmiReceiveError::EmptyMessage);
        };

        Ok(response)
    }

    fn send_recv(&mut self, request: &mut Request) -> Result<Response, Self::Error> {
        self.send(request)?;
        let response = self.recv()?;
        Ok(response)
    }
}
