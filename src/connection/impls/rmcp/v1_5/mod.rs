use std::{net::UdpSocket, num::NonZeroU32};

use crate::{
    app::auth::{
        ActivateSession, AuthError, AuthType, ChannelAuthenticationCapabilities,
        GetSessionChallenge, PrivilegeLevel,
    },
    connection::{
        rmcp::{IpmiSessionMessage, RmcpMessage},
        IpmiConnection, ParseResponseError, Request, Response,
    },
    Ipmi, IpmiError,
};

use super::{internal::IpmbState, RmcpClass, RmcpError, RmcpReceiveError};

pub use message::Message;

mod auth;
mod checksum;
mod md2;
mod message;

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
    session_sequence: u32,
}

impl State {
    pub fn new(socket: UdpSocket) -> Self {
        Self {
            socket,
            ipbm_state: Default::default(),
            auth_type: AuthType::None,
            password: None,
            session_id: None,
            session_sequence: 0,
        }
    }

    pub fn release_socket(self) -> UdpSocket {
        self.socket
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
    type SendError = RmcpError;

    type RecvError = RmcpError;

    type Error = RmcpError;

    fn send(&mut self, request: &mut Request) -> Result<(), RmcpError> {
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

        let message: RmcpMessage = IpmiSessionMessage::V1_5(Message {
            auth_type: self.auth_type,
            session_sequence_number: self.session_sequence,
            session_id: self.session_id.map(|v| v.get()).unwrap_or(0),
            payload: final_data,
        })
        .into();

        let send_bytes = message.to_bytes(self.password.as_ref())?;

        self.socket
            .send(&send_bytes)
            .map_err(Into::into)
            .map(|_| ())
    }

    fn recv(&mut self) -> Result<Response, RmcpError> {
        let mut buffer = [0u8; 1024];
        let received_bytes = self.socket.recv(&mut buffer)?;

        let data = &buffer[..received_bytes];

        let rcmp_message = RmcpMessage::from_bytes(self.password.as_ref(), data)
            .map_err(RmcpReceiveError::Rmcp)?;

        let encapsulated_message = if let RmcpMessage {
            class_and_contents: RmcpClass::Ipmi(message),
            ..
        } = rcmp_message
        {
            message
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "RMCP response does not have IPMI class",
            )
            .into());
        };

        let data = match encapsulated_message {
            IpmiSessionMessage::V1_5(Message { payload, .. }) => payload,
            IpmiSessionMessage::V2_0 { .. } => todo!(),
        };

        if data.len() < 7 {
            return Err(RmcpReceiveError::NotEnoughData.into());
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
            return Err(
                std::io::Error::new(std::io::ErrorKind::Other, "Response data was empty").into(),
            );
        };

        Ok(response)
    }

    fn send_recv(&mut self, request: &mut Request) -> Result<Response, Self::Error> {
        self.send(request)?;
        self.recv()
    }
}
