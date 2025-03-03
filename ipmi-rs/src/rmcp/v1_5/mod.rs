use std::num::NonZeroU32;

use crate::{
    app::auth::{
        ActivateSession, AuthType, ChannelAuthenticationCapabilities, GetSessionChallenge,
        PrivilegeLevel,
    },
    connection::{Request, Response},
};

use super::{
    internal::{validate_ipmb_checksums, IpmbState},
    RmcpIpmiReceiveError,
};

use ipmi_rs_core::{app::auth::AuthError, connection::IpmiCommand};
pub use message::Message;

mod auth;
mod md2;
#[cfg(feature = "md5")]
mod md5;
mod message;

#[derive(Debug)]
pub enum ActivationError {
    Io(std::io::Error),
    PasswordTooLong,
    UsernameTooLong,
    GetSessionChallenge(WriteError),
    ParseSessionChallenge(AuthError),
    ReceiveSessionChallenge(RmcpIpmiReceiveError),
    ReceiveSessionInfo(RmcpIpmiReceiveError),
    ParseSessionInfo(AuthError),
    NoSupportedAuthenticationType,
    ActivateSession(WriteError),
}

#[derive(Debug)]
pub enum WriteError {
    /// A request was made to calculate the auth code for a message authenticated
    /// using a method that requires a password, but no password was provided.
    MissingPassword,
    /// The payload length of for the V1_5 packet is larger than the maximum
    /// allowed size (256 bytes).
    PayloadTooLarge(usize),
    /// The requested auth type is not supported.
    UnsupportedAuthType(AuthType),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReadError {
    /// There is not enough data in the packet to form a valid `Message`.
    NotEnoughData,
    /// The auth type provided is not supported.
    UnsupportedAuthType(u8),
    /// There is a mismatch between the payload length field and the
    /// actual length of the payload.
    IncorrectPayloadLen,
    /// The auth code of the message is not correct.
    AuthcodeError,
}

#[derive(Debug, Clone, Copy)]
pub struct Inactive;

#[derive(Debug)]
pub struct SessionChallengeSent;

#[derive(Debug)]
pub struct ActivationSent {
    auth_type: AuthType,
}

#[derive(Debug)]
pub struct Active;

#[derive(Debug, Clone)]
pub struct State<T = Active> {
    ipmb_state: IpmbState,
    session_id: Option<NonZeroU32>,
    auth_type: crate::app::auth::AuthType,
    password: Option<[u8; 16]>,
    session_sequence: u32,
    activation_state: T,
}

impl<T> State<T> {
    fn with_state<TNext>(self, state: TNext) -> State<TNext> {
        State {
            ipmb_state: self.ipmb_state,
            session_id: self.session_id,
            auth_type: self.auth_type,
            password: self.password,
            session_sequence: self.session_sequence,
            activation_state: state,
        }
    }

    pub(crate) fn send<M>(&mut self, request: M) -> Result<Vec<u8>, WriteError>
    where
        M: Into<Request>,
    {
        let mut request = request.into();
        log::trace!("Sending message with auth type {:?}", self.auth_type);

        let request_sequence = &mut self.session_sequence;

        // Only increment the request sequence once a session has been established
        // succesfully.
        if self.session_id.is_some() {
            *request_sequence = request_sequence.wrapping_add(1);
        }

        let final_data = super::internal::next_ipmb_message(&mut request, &mut self.ipmb_state);

        let message = Message {
            auth_type: self.auth_type,
            session_sequence_number: self.session_sequence,
            session_id: self.session_id.map(|v| v.get()).unwrap_or(0),
            payload: final_data,
        };

        super::write_ipmi_data(|buffer| message.write_data(self.password.as_ref(), buffer))
    }

    pub(crate) fn recv(&mut self, data: &mut [u8]) -> Result<Response, RmcpIpmiReceiveError> {
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

        if let Some(resp) = Response::new(netfn, cmd, response_data, 0) {
            Ok(resp)
        } else {
            // TODO: need better message here :)
            Err(RmcpIpmiReceiveError::EmptyMessage)
        }
    }
}

impl State<Inactive> {
    pub fn new() -> Self {
        Self {
            ipmb_state: Default::default(),
            auth_type: AuthType::None,
            password: None,
            session_id: None,
            session_sequence: 0,
            activation_state: Inactive,
        }
    }

    pub fn activate(
        mut self,
        username: Option<&str>,
        password: Option<&[u8]>,
    ) -> Result<(State<SessionChallengeSent>, Vec<u8>), ActivationError> {
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

        log::debug!("Requesting challenge");

        let challenge_command = match GetSessionChallenge::new(AuthType::None, username) {
            Some(v) => v,
            None => return Err(ActivationError::UsernameTooLong),
        };

        let data: Vec<u8> = self
            .send(challenge_command)
            .map_err(ActivationError::GetSessionChallenge)?;

        Ok((self.with_state(SessionChallengeSent), data))
    }
}

impl State<SessionChallengeSent> {
    pub fn recv_session_challenge(
        mut self,
        challenge_packet: &mut [u8],
        authentication_caps: &ChannelAuthenticationCapabilities,
        privilege_level: PrivilegeLevel,
    ) -> Result<(State<ActivationSent>, Vec<u8>), ActivationError> {
        let challenge = self
            .recv(challenge_packet)
            .map_err(ActivationError::ReceiveSessionChallenge)?;

        let challenge = GetSessionChallenge::parse_success_response(challenge.data())
            .map_err(ActivationError::ParseSessionChallenge)?;

        let activation_auth_type = authentication_caps
            .best_auth()
            .ok_or(ActivationError::NoSupportedAuthenticationType)?;

        let activate_session: ActivateSession = ActivateSession {
            auth_type: activation_auth_type,
            maxiumum_privilege_level: privilege_level,
            challenge_string: challenge.challenge_string,
            initial_sequence_number: 0xDEAD_BEEF,
        };

        self.session_id = Some(challenge.temporary_session_id);
        self.auth_type = activation_auth_type;

        log::debug!("Activating session");

        let message = self
            .send(activate_session)
            .map_err(ActivationError::ActivateSession)?;

        let next = self.with_state(ActivationSent {
            auth_type: activation_auth_type,
        });

        Ok((next, message))
    }
}

impl State<ActivationSent> {
    pub fn recv_session_info(mut self, data: &mut [u8]) -> Result<State<Active>, ActivationError> {
        let session_info = self
            .recv(data)
            .map_err(ActivationError::ReceiveSessionInfo)?;

        let activation_info = ActivateSession::parse_success_response(session_info.data())
            .map_err(ActivationError::ParseSessionInfo)?;

        log::debug!("Succesfully started a session ({:?})", activation_info);

        self.session_sequence = activation_info.initial_sequence_number;
        self.session_id = Some(activation_info.session_id);

        assert_eq!(activation_info.auth_type, self.activation_state.auth_type);

        Ok(self.with_state(Active))
    }
}
