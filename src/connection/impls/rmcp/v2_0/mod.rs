use std::{
    io::{Error, ErrorKind},
    net::UdpSocket,
    num::NonZeroU32,
};

use crate::{
    app::auth::PrivilegeLevel,
    connection::rmcp::{
        socket::RmcpIpmiSocket,
        v2_0::{
            open_session::OpenSessionResponse,
            rakp_1_2::{RakpMessageOne, RakpMessageTwo, Username},
        },
    },
};

use self::open_session::OpenSessionRequest;

mod open_session;

mod crypto;
mod rakp_1_2;
pub use crypto::{
    Algorithm, AuthenticationAlgorithm, ConfidentialityAlgorithm, CryptoState, IntegrityAlgorithm,
};

use super::{v1_5, IpmiSessionMessage};

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
    ) -> Result<(), &'static str> {
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

    pub fn from_data(state: &mut CryptoState, data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < 10 {
            return Err("Not enough data");
        }

        if data[0] != 0x06 {
            return Err("Not an RMCP+ packet");
        }

        let encrypted = (data[1] & 0x80) == 0x80;
        let authenticated = (data[1] & 0x40) == 0x40;
        let ty = PayloadType::try_from(data[1] & 0x3F).map_err(|_| "Invalid payload type")?;

        let session_id = u32::from_le_bytes(data[2..6].try_into().unwrap());
        let session_sequence_number = u32::from_le_bytes(data[6..10].try_into().unwrap());

        let payload = state.read_payload(encrypted, authenticated, &data[10..])?;

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
    socket: UdpSocket,
    session_id: Option<NonZeroU32>,
    session_sequence_number: Option<NonZeroU32>,
    state: CryptoState,
}

impl State {
    pub fn new(socket: UdpSocket) -> Self {
        Self {
            socket,
            session_id: None,
            session_sequence_number: None,
            state: CryptoState::default(),
        }
    }

    fn send_v2_message(
        crypto_state: &mut CryptoState,
        socket: &mut RmcpIpmiSocket,
        ty: PayloadType,
        payload: Vec<u8>,
    ) -> std::io::Result<()> {
        let message = Message {
            ty,
            session_id: 0,
            session_sequence_number: 0,
            payload,
        };

        socket.send(|buffer| {
            message
                .write_data(crypto_state, buffer)
                .map_err(|e| Error::new(ErrorKind::Other, e))
        })
    }

    fn get_v2_message(data: &[u8]) -> std::io::Result<Message> {
        match IpmiSessionMessage::from_data(data, None) {
            Ok(IpmiSessionMessage::V2_0(message)) => Ok(message),
            e => {
                return Err(Error::new(
                    ErrorKind::Other,
                    format!("Expected IPMI V2.0 message, got {e:?}"),
                ))
            }
        }
    }

    // TODO: validate message tags
    // TODO: validate sequence numbers
    // TODO: validate remote console session ID
    // TODO: validate managed system session ID
    // TODO: assert that rng is always CryptoRng
    pub fn activate(
        state: v1_5::State,
        privilege_level: Option<PrivilegeLevel>,
    ) -> std::io::Result<Self> {
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

        let mut crypto_state = CryptoState::default();

        let mut payload = Vec::new();
        open_session_request.write_data(&mut payload);
        Self::send_v2_message(
            &mut crypto_state,
            &mut socket,
            PayloadType::RmcpPlusOpenSessionRequest,
            payload,
        )?;

        let data = socket
            .recv()
            .map_err(|e| Error::new(ErrorKind::Other, format!("{e:?}")))?;

        let response =
            OpenSessionResponse::from_data(&Self::get_v2_message(data)?.payload).unwrap();

        log::debug!("Received RMCP+ Open Session Response: {response:?}.");

        let username = &Username::new("jona").unwrap();

        use rand::Rng;
        let mut rng = rand::thread_rng();
        let random_data = rng.gen();

        let rakp_message_1 = RakpMessageOne {
            message_tag: 0x0D,
            managed_system_session_id: response.managed_system_session_id,
            remote_console_random_number: random_data,
            requested_maximum_privilege_level: PrivilegeLevel::Administrator,
            username,
        };

        let mut payload = Vec::new();
        rakp_message_1.write(&mut payload);

        log::debug!("Sending RMCP+ RAKP Message 1.");

        Self::send_v2_message(
            &mut crypto_state,
            &mut socket,
            PayloadType::RakpMessage1,
            payload,
        )?;

        let data = socket
            .recv()
            .map_err(|e| Error::new(ErrorKind::Other, format!("{e:?}")))?;

        let v2_message = Self::get_v2_message(data)?;
        let rakp_message_2 = RakpMessageTwo::from_data(&v2_message.payload).unwrap();

        log::debug!("Received RMCP+ RAKP Message 2. {rakp_message_2:X?}");

        let kex_auth_code = rakp_message_2.key_exchange_auth_code;

        let required_kex_auth_code_len = match response.authentication_payload {
            AuthenticationAlgorithm::RakpNone => 0,
            AuthenticationAlgorithm::RakpHmacSha1 => 20,
            AuthenticationAlgorithm::RakpHmacSha256 => 32,
            AuthenticationAlgorithm::RakpHmacMd5 => 16,
        };

        if kex_auth_code.len() != required_kex_auth_code_len {
            return Err(Error::new(
                ErrorKind::Other,
                format!(
                    "Key exchange auth code length {} is incorrect for authentication algorithm {:?}",
                    kex_auth_code.len(),
                    response.authentication_payload
                ),
            ));
        }

        let configured_crypto_state = crypto_state.configured(b"password", &response);
        configured_crypto_state.validate(&rakp_message_1, &rakp_message_2);

        Ok(State::new(socket.release()))
    }
}
