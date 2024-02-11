use std::{
    io::{Error, ErrorKind},
    net::UdpSocket,
    num::NonZeroU32,
};

use crate::{
    app::auth::{ChannelCipherSuites, GetChannelCipherSuites, PrivilegeLevel},
    connection::{
        rmcp::{v2_0::open_session::OpenSessionResponse, RmcpType},
        Channel,
    },
    Ipmi,
};

use self::open_session::OpenSessionRequest;

mod open_session;

mod crypto;
pub use crypto::{
    Algorithm, AuthenticationAlgorithm, ConfidentialityAlgorithm, CryptoState, IntegrityAlgorithm,
};

use super::{v1_5, IpmiSessionMessage, RmcpHeader};

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

    pub fn activate(
        state: v1_5::State,
        privilege_level: Option<PrivilegeLevel>,
    ) -> std::io::Result<Self> {
        let me = Self::new(state.release_socket());

        let open_session_request = OpenSessionRequest {
            message_tag: 0,
            requested_max_privilege: privilege_level,
            remote_console_session_id: 0x0AA2A3A4,
            // Writing NULL byte seems to be badly
            // supported , and writing more than
            // one payload seems to give some IPMI devices
            // the shits, so we only pick a single default.
            //
            // TODO: open a few sessions to see what the best
            // we can do is, in parallel?
            authentication_algorithms: vec![Default::default()],
            confidentiality_algorithms: vec![Default::default()],
            integrity_algorithms: vec![Default::default()],
        };

        let mut payload = Vec::new();
        open_session_request.write_data(&mut payload);

        let header = RmcpHeader::new_ipmi();

        let message = Message {
            ty: PayloadType::RmcpPlusOpenSessionRequest,
            session_id: 0,
            session_sequence_number: 0,
            payload,
        };

        let data = header
            .write(|buffer| message.write_data(&mut CryptoState::default(), buffer))
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        log::debug!("Sending RMCP+ Open Session Request.");

        me.socket.send(&data).unwrap();

        let mut buffer = [0u8; 1024];
        let recvd = me.socket.recv(&mut buffer)?;
        let recvd = &buffer[..recvd];

        let (message, data) = RmcpHeader::from_bytes(&recvd)
            .map_err(|e| Error::new(ErrorKind::Other, format!("{e:?}")))?;

        // TODO: validate payload type, session id == 0, session sequence number == 0
        // TODO: validate message_tag is correct

        if message.class().ty != RmcpType::Ipmi {
            return Err(Error::new(
                ErrorKind::Other,
                "Received non-IPMI response to open session request",
            ));
        }

        let message = match IpmiSessionMessage::from_data(data, None) {
            Ok(IpmiSessionMessage::V2_0(message)) => message,
            e => {
                return Err(Error::new(
                    ErrorKind::Other,
                    format!("Expected IPMI V2.0 message, got {e:?}"),
                ))
            }
        };

        let response = OpenSessionResponse::from_data(&message.payload);

        println!("{response:?}");

        Ok(me)
    }
}
