use std::{num::NonZeroU32, ops::Add};

use crate::{
    app::auth::PrivilegeLevel,
    connection::{rmcp::socket::RmcpIpmiSocket, Response},
};

mod crypto;
use crypto::CryptoState;
pub use crypto::{AuthenticationAlgorithm, ConfidentialityAlgorithm, IntegrityAlgorithm};

mod messages;
pub(super) use messages::*;
pub use messages::{
    ParseSessionResponseError, RakpMessage2ErrorStatusCode, RakpMessage2ParseError,
    RakpMessage4ErrorStatusCode, RakpMessage4ParseError,
};

use self::crypto::CryptoUnwrapError;

use super::{internal::IpmbState, v1_5, RmcpIpmiError, RmcpIpmiReceiveError, UnwrapSessionError};

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
    RakpMessage4InvalidIntegrityCheckValue,
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
    EncryptedPayloadTooLong,
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

impl From<CryptoUnwrapError> for ReadError {
    fn from(value: CryptoUnwrapError) -> Self {
        Self::DecryptionError(value)
    }
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

#[derive(Debug, Clone)]
pub struct Message {
    pub ty: PayloadType,
    pub session_id: u32,
    pub session_sequence_number: u32,
    pub payload: Vec<u8>,
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

            socket.send(|buffer| CryptoState::write_unencrypted(&message, buffer))
        }

        fn recv(data: &mut [u8]) -> Result<Message, UnwrapSessionError> {
            CryptoState::default()
                .read_payload(data)
                .map_err(UnwrapSessionError::V2_0)
        }

        let mut socket = RmcpIpmiSocket::new(state.release_socket());

        let open_session_request = OpenSessionRequest {
            message_tag: 0,
            requested_max_privilege: privilege_level,
            remote_console_session_id: 0x0AA2A3A4,
            authentication_algorithms: AuthenticationAlgorithm::RakpHmacSha1,
            confidentiality_algorithms: ConfidentialityAlgorithm::AesCbc128,
            integrity_algorithms: IntegrityAlgorithm::HmacSha1_96,
        };

        log::debug!("Sending RMCP+ Open Session Request. {open_session_request:?}");

        let mut payload = Vec::new();
        open_session_request.write_data(&mut payload);
        send(
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

        let rm1 = RakpMessage1 {
            message_tag: 0x0D,
            managed_system_session_id: response.managed_system_session_id,
            remote_console_random_number: random_data,
            requested_maximum_privilege_level: PrivilegeLevel::Administrator,
            username,
        };

        let mut payload = Vec::new();
        rm1.write(&mut payload);

        log::debug!("Sending RMCP+ RAKP Message 1.");

        send(&mut socket, PayloadType::RakpMessage1, payload)
            .map_err(ActivationError::SendRakpMessage1)?;

        let data = socket
            .recv()
            .map_err(ActivationError::RakpMessage2Receive)?;

        let v2_message = recv(data).map_err(ActivationError::RakpMessage2Read)?;
        let rm2 = RakpMessage2::from_data(&v2_message.payload)
            .map_err(ActivationError::RakpMessage2Parse)?;

        log::debug!("Received RMCP+ RAKP Message 2. {rm2:X?}");

        let kex_auth_code = rm2.key_exchange_auth_code;

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

        let mut crypto_state = CryptoState::new(None, password);
        let message_3_value = crypto_state.calculate_rakp3_data(&response, &rm1, &rm2);

        let rm3 = if let Some(m3) = message_3_value.as_ref() {
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
        rm3.write(&mut payload);

        log::debug!("Sending RAKP message 3. {rm3:?}");

        send(&mut socket, PayloadType::RakpMessage3, payload)
            .map_err(ActivationError::RakpMessage3Send)?;

        if rm3.is_failure() {
            return Err(ActivationError::ServerAuthenticationFailed)?;
        }

        let data = socket
            .recv()
            .map_err(ActivationError::RakpMessage4Receive)?;

        let message = recv(data).unwrap();
        let rm4 = RakpMessage4::from_data(&message.payload)
            .map_err(ActivationError::RakpMessage4Parse)?;

        log::debug!("Received RAKP Message 4: {rm4:02X?}");

        if !crypto_state.verify(
            response.authentication_payload,
            &rm1.remote_console_random_number,
            rm3.managed_system_session_id.get(),
            &rm2.managed_system_guid,
            rm4.integrity_check_value,
        ) {
            log::error!("Received incorrect/invalid integrity check value in RAKP Message 4.");
            return Err(ActivationError::RakpMessage4InvalidIntegrityCheckValue);
        }

        let session_id = rm3.managed_system_session_id;
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

        let payload = super::internal::next_ipmb_message(request, &mut self.ipmb_state);

        let message = Message {
            ty: PayloadType::IpmiMessage,
            session_id: self.session_id.get(),
            session_sequence_number,
            payload,
        };

        self.socket
            .send(|buffer| self.state.write_message(&message, buffer))
            .unwrap();

        Ok(())
    }

    // TODO: validate session sequence number
    // TODO: Validate session sequence ID
    pub fn recv(&mut self) -> Result<crate::connection::Response, RmcpIpmiReceiveError> {
        let data = self.socket.recv()?;

        let data = self
            .state
            .read_payload(data)
            .map_err(|e| RmcpIpmiReceiveError::Session(UnwrapSessionError::V2_0(e)))?
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
        let response_data: Vec<_> = data[6..data.len()].to_vec();
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

    pub fn send_recv(
        &mut self,
        request: &mut crate::connection::Request,
    ) -> Result<crate::connection::Response, RmcpIpmiError> {
        self.send(request)?;
        self.recv().map_err(Into::into)
    }
}
