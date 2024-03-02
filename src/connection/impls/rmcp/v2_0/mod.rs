use std::{net::UdpSocket, num::NonZeroU32, ops::Add};

use crate::{app::auth::PrivilegeLevel, connection::rmcp::socket::RmcpIpmiSocket};

mod crypto;
pub use crypto::{
    Algorithm, AuthenticationAlgorithm, ConfidentialityAlgorithm, CryptoState, IntegrityAlgorithm,
};

mod messages;
pub(super) use messages::*;
pub use messages::{
    ParseSessionResponseError, RakpMessage2ErrorStatusCode, RakpMessage2ParseError,
    RakpMessage4ErrorStatusCode, RakpMessage4ParseError,
};

use self::crypto::CryptoUnwrapError;

use super::{
    internal::IpmbState, v1_5, IpmiSessionMessage, RmcpIpmiError, RmcpIpmiReceiveError,
    UnwrapSessionError,
};

#[derive(Debug)]
pub enum ActivationError {
    Io(std::io::Error),
    InvalidKeyExchangeAuthCodeLen(usize, AuthenticationAlgorithm),
    OpenSessionRequestSend(WriteError),
    OpenSessionResponseReceive(RmcpIpmiReceiveError),
    OpenSessionResponseRead(UnwrapSessionError),
    OpenSessionResponseParse(ParseSessionResponseError),
    SendRakpMessage1(WriteError),
    RakpMessage2Receive(RmcpIpmiReceiveError),
    RakpMessage2Read(UnwrapSessionError),
    RakpMessage2Parse(RakpMessage2ParseError),
    RakpMessage3Send(WriteError),
    RakpMessage4Receive(RmcpIpmiReceiveError),
    RakpMessage4Read(UnwrapSessionError),
    RakpMessage4Parse(RakpMessage4ParseError),
    ServerAuthenticationFailed,
}

impl From<ParseSessionResponseError> for ActivationError {
    fn from(value: ParseSessionResponseError) -> Self {
        Self::OpenSessionResponseParse(value)
    }
}

#[derive(Debug)]
pub enum WriteError {
    Io(std::io::Error),
    PayloadTooLong,
}

impl From<std::io::Error> for WriteError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReadError {
    NotIpmiV2_0,
    NotEnoughData,
    NotRmcpPlus,
    InvalidPayloadType(u8),
    DecryptionError(CryptoUnwrapError),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PayloadType {
    IpmiMessage,
    Sol,
    RmcpPlusOpenSessionRequest,
    RmcpPlusOpenSessionResponse,
    RakpMessage1,
    RakpMessage2,
    RakpMessage3,
    RakpMessage4,
}

impl TryFrom<u8> for PayloadType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let ty = match value {
            0x00 => PayloadType::IpmiMessage,
            0x01 => PayloadType::Sol,
            0x10 => PayloadType::RmcpPlusOpenSessionRequest,
            0x11 => PayloadType::RmcpPlusOpenSessionResponse,
            0x12 => PayloadType::RakpMessage1,
            0x13 => PayloadType::RakpMessage2,
            0x14 => PayloadType::RakpMessage3,
            0x15 => PayloadType::RakpMessage4,
            _ => return Err(()),
        };

        Ok(ty)
    }
}

impl From<PayloadType> for u8 {
    fn from(value: PayloadType) -> Self {
        match value {
            PayloadType::IpmiMessage => 0x00,
            PayloadType::Sol => 0x01,
            PayloadType::RmcpPlusOpenSessionRequest => 0x10,
            PayloadType::RmcpPlusOpenSessionResponse => 0x11,
            PayloadType::RakpMessage1 => 0x12,
            PayloadType::RakpMessage2 => 0x13,
            PayloadType::RakpMessage3 => 0x14,
            PayloadType::RakpMessage4 => 0x15,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Message {
    ty: PayloadType,
    session_id: u32,
    session_sequence_number: u32,
    payload: Vec<u8>,
}

impl Message {
    pub fn write_data(
        &self,
        state: &mut CryptoState,
        buffer: &mut Vec<u8>,
    ) -> Result<(), WriteError> {
        buffer.push(0x06);

        let encrypted = (state.encrypted() as u8) << 7;
        let authenticated = (state.authenticated() as u8) << 6;
        buffer.push(encrypted | authenticated | u8::from(self.ty));

        // TODO: support OEM IANA and OEM payload ID? Ignore for now: unsupported payload type

        buffer.extend_from_slice(&self.session_id.to_le_bytes());
        buffer.extend_from_slice(&self.session_sequence_number.to_le_bytes());

        state.write_payload(&self.payload, buffer)?;

        Ok(())
    }

    pub fn from_data(state: &mut CryptoState, data: &[u8]) -> Result<Self, ReadError> {
        if data.len() < 10 {
            return Err(ReadError::NotEnoughData);
        }

        if data[0] != 0x06 {
            return Err(ReadError::NotIpmiV2_0);
        }

        let encrypted = (data[1] & 0x80) == 0x80;
        let authenticated = (data[1] & 0x40) == 0x40;
        let ty = PayloadType::try_from(data[1] & 0x3F)
            .map_err(|_| ReadError::InvalidPayloadType(data[1] & 0x3F))?;

        let session_id = u32::from_le_bytes(data[2..6].try_into().unwrap());
        let session_sequence_number = u32::from_le_bytes(data[6..10].try_into().unwrap());

        let payload = state
            .read_payload(encrypted, authenticated, &data[10..])
            .map_err(ReadError::DecryptionError)?;

        Ok(Self {
            ty,
            session_id,
            session_sequence_number,
            payload,
        })
    }
}

#[derive(Debug)]
pub struct State {
    socket: RmcpIpmiSocket,
    session_id: NonZeroU32,
    session_sequence_number: NonZeroU32,
    state: CryptoState,
    ipmb_state: IpmbState,
}

impl State {
    // TODO: validate message tags
    // TODO: validate sequence numbers
    // TODO: validate remote console session ID
    // TODO: validate managed system session ID
    // TODO: validate RAKP message 4
    // TODO: assert that rng is always CryptoRng
    pub fn activate(
        state: v1_5::State,
        privilege_level: Option<PrivilegeLevel>,
        username: &Username,
        password: &[u8],
    ) -> Result<Self, ActivationError> {
        fn send(
            crypto_state: &mut CryptoState,
            socket: &mut RmcpIpmiSocket,
            ty: PayloadType,
            payload: Vec<u8>,
        ) -> Result<(), WriteError> {
            let message = Message {
                ty,
                session_id: 0,
                session_sequence_number: 0,
                payload,
            };

            socket.send(|buffer| message.write_data(crypto_state, buffer))
        }

        fn recv(data: &[u8]) -> Result<Message, UnwrapSessionError> {
            match IpmiSessionMessage::from_data(data, None) {
                Ok(IpmiSessionMessage::V2_0(message)) => Ok(message),
                Ok(IpmiSessionMessage::V1_5(_)) => {
                    Err(UnwrapSessionError::V2_0(ReadError::NotIpmiV2_0))
                }
                Err(e) => Err(e),
            }
        }

        let mut socket = RmcpIpmiSocket::new(state.release_socket());

        let open_session_request = OpenSessionRequest {
            message_tag: 0,
            requested_max_privilege: privilege_level,
            remote_console_session_id: 0x0AA2A3A4,
            authentication_algorithms: None,
            confidentiality_algorithms: None,
            integrity_algorithms: None,
        };

        log::debug!("Sending RMCP+ Open Session Request.");

        let mut inactive_crypto_state = CryptoState::default();

        let mut payload = Vec::new();
        open_session_request.write_data(&mut payload);
        send(
            &mut inactive_crypto_state,
            &mut socket,
            PayloadType::RmcpPlusOpenSessionRequest,
            payload,
        )
        .map_err(ActivationError::OpenSessionRequestSend)?;

        let data = socket
            .recv()
            .map_err(ActivationError::OpenSessionResponseReceive)?;

        let response_data = recv(data).map_err(|e| ActivationError::OpenSessionResponseRead(e))?;

        let response = match OpenSessionResponse::from_data(&response_data.payload) {
            Ok(r) => r,
            Err(ParseSessionResponseError::HaveErrorCode(error_code)) => {
                log::warn!("RMCP+ error occurred. Status code: '{error_code:?}'");
                return Err(ParseSessionResponseError::HaveErrorCode(error_code).into());
            }
            Err(e) => return Err(e.into()),
        };

        log::debug!("Received RMCP+ Open Session Response: {response:?}.");

        use rand::Rng;
        let mut rng = rand::thread_rng();
        let random_data = rng.gen();

        let rakp_message_1 = RakpMessage1 {
            message_tag: 0x0D,
            managed_system_session_id: response.managed_system_session_id,
            remote_console_random_number: random_data,
            requested_maximum_privilege_level: PrivilegeLevel::Administrator,
            username,
        };

        let mut payload = Vec::new();
        rakp_message_1.write(&mut payload);

        log::debug!("Sending RMCP+ RAKP Message 1.");

        send(
            &mut inactive_crypto_state,
            &mut socket,
            PayloadType::RakpMessage1,
            payload,
        )
        .map_err(ActivationError::SendRakpMessage1)?;

        let data = socket
            .recv()
            .map_err(ActivationError::RakpMessage2Receive)?;

        let v2_message = recv(data).map_err(ActivationError::RakpMessage2Read)?;
        let rakp_message_2 = RakpMessage2::from_data(&v2_message.payload)
            .map_err(ActivationError::RakpMessage2Parse)?;

        log::debug!("Received RMCP+ RAKP Message 2. {rakp_message_2:X?}");

        let kex_auth_code = rakp_message_2.key_exchange_auth_code;

        let required_kex_auth_code_len = match response.authentication_payload {
            AuthenticationAlgorithm::RakpNone => 0,
            AuthenticationAlgorithm::RakpHmacSha1 => 20,
            AuthenticationAlgorithm::RakpHmacSha256 => 32,
            AuthenticationAlgorithm::RakpHmacMd5 => 16,
        };

        if kex_auth_code.len() != required_kex_auth_code_len {
            return Err(ActivationError::InvalidKeyExchangeAuthCodeLen(
                kex_auth_code.len(),
                response.authentication_payload,
            ));
        }

        let mut crypto_state = CryptoState::new(None, password, &response);
        let message_3_value = crypto_state.validate(&rakp_message_1, &rakp_message_2);

        let rakp_message_3 = if let Some(m3) = message_3_value.as_ref() {
            RakpMessage3 {
                message_tag: 0x0A,
                managed_system_session_id: response.managed_system_session_id,
                contents: RakpMessage3Contents::Succes(m3),
            }
        } else {
            log::warn!("Received RAKP message 2 with invalid integrity check value.");

            RakpMessage3 {
                message_tag: 0x0A,
                managed_system_session_id: response.managed_system_session_id,
                contents: RakpMessage3Contents::Failure(
                    RakpMessage3ErrorStatusCode::InvalidIntegrityCheckValue,
                ),
            }
        };

        let mut payload = Vec::new();
        rakp_message_3.write(&mut payload);

        log::debug!("Sending RAKP message 3.");

        send(
            &mut inactive_crypto_state,
            &mut socket,
            PayloadType::RakpMessage3,
            payload,
        )
        .map_err(ActivationError::RakpMessage3Send)?;

        if rakp_message_3.is_failure() {
            return Err(ActivationError::ServerAuthenticationFailed)?;
        }

        let data = socket
            .recv()
            .map_err(ActivationError::RakpMessage4Receive)?;

        let message = recv(data).unwrap();
        let rakp_message_4 = RakpMessage4::from_data(&message.payload)
            .map_err(ActivationError::RakpMessage4Parse)?;

        let session_id = rakp_message_2.remote_console_session_id;
        let session_sequence_number = NonZeroU32::new(1).unwrap();

        Ok(Self {
            socket,
            session_id,
            session_sequence_number,
            state: crypto_state,
            ipmb_state: IpmbState::default(),
        })
    }

    pub fn send(&mut self, request: &mut crate::connection::Request) -> Result<(), RmcpIpmiError> {
        let session_sequence_number = self.session_sequence_number.get();

        if session_sequence_number == u32::MAX {
            todo!("Handle wrapping session number by re-activating?");
        }

        self.session_sequence_number =
            NonZeroU32::new(self.session_sequence_number.get().add(1)).unwrap();

        let payload_data = super::internal::next_ipmb_message(request, &mut self.ipmb_state);

        let mut payload = Vec::new();

        self.state
            .write_payload(&payload_data, &mut payload)
            .map_err(|e| RmcpIpmiError::Send(super::RmcpIpmiSendError::V2_0(e)))?;

        let message = Message {
            ty: PayloadType::IpmiMessage,
            session_id: self.session_id.get(),
            session_sequence_number: self.session_sequence_number.get(),
            payload,
        };

        self.socket
            .send(|buffer| message.write_data(&mut self.state, buffer))
            .unwrap();

        Ok(())
    }
}
