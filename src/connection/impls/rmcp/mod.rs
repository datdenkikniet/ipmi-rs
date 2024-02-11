use crate::{connection::IpmiConnection, IpmiCommandError};
use std::{net::ToSocketAddrs, time::Duration};

mod v1_5;
use v1_5::Message as V1_5Message;
pub use v1_5::{
    ActivationError as V1_5ActivationError, ReadError as V1_5ReadError,
    WriteError as V1_5WriteError,
};

mod v2_0;
use v2_0::Message as V2_0Message;
pub use v2_0::{Algorithm, AuthenticationAlgorithm, ConfidentialityAlgorithm, IntegrityAlgorithm};

mod header;
pub(crate) use header::*;

mod asf;
pub(crate) use asf::*;

mod internal;
use internal::{Active, RmcpWithState, Unbound};

#[derive(Debug)]
pub enum RmcpReceiveError {
    /// An RMCP error occured.
    Rmcp(RmcpUnwrapError),
    /// Invalid IPMI data
    InvalidPayloadData(ReadError),
    /// The packet did not contain enough data to form a valid RMCP message.
    NotEnoughData,
}

impl From<ReadError> for RmcpReceiveError {
    fn from(value: ReadError) -> Self {
        Self::InvalidPayloadData(value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum WriteError {
    V1_5(V1_5WriteError),
    V2_0(&'static str),
}

impl From<V1_5WriteError> for WriteError {
    fn from(value: V1_5WriteError) -> Self {
        Self::V1_5(value)
    }
}

impl From<&'static str> for WriteError {
    fn from(value: &'static str) -> Self {
        Self::V2_0(value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReadError {
    V1_5(V1_5ReadError),
    V2_0(&'static str),
}

impl From<V1_5ReadError> for ReadError {
    fn from(value: V1_5ReadError) -> Self {
        Self::V1_5(value)
    }
}

impl From<&'static str> for ReadError {
    fn from(value: &'static str) -> Self {
        Self::V2_0(value)
    }
}

#[derive(Clone, Debug)]
pub enum IpmiSessionMessage {
    V1_5(V1_5Message),
    V2_0(V2_0Message),
}

impl IpmiSessionMessage {
    pub fn write_data(
        &self,
        password: Option<&[u8; 16]>,
        buffer: &mut Vec<u8>,
    ) -> Result<(), WriteError> {
        match self {
            IpmiSessionMessage::V1_5(message) => {
                message.write_data(password, buffer).map_err(Into::into)
            }
            IpmiSessionMessage::V2_0(message) => message
                .write_data(&mut v2_0::CryptoState::default(), buffer)
                .map_err(Into::into),
        }
    }

    pub fn from_data(data: &[u8], password: Option<&[u8; 16]>) -> Result<Self, ReadError> {
        if data[0] != 0x06 {
            Ok(Self::V1_5(V1_5Message::from_data(password, data)?))
        } else {
            Ok(Self::V2_0(V2_0Message::from_data(
                &mut v2_0::CryptoState::default(),
                data,
            )?))
        }
    }
}

#[derive(Debug)]
pub enum RmcpError {
    NotActive,
    Io(std::io::Error),
    Receive(RmcpReceiveError),
    Send(WriteError),
}

impl From<RmcpReceiveError> for RmcpError {
    fn from(value: RmcpReceiveError) -> Self {
        Self::Receive(value)
    }
}

impl From<WriteError> for RmcpError {
    fn from(value: WriteError) -> Self {
        Self::Send(value)
    }
}

impl From<std::io::Error> for RmcpError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

type CommandError<T> = IpmiCommandError<RmcpError, T>;

#[derive(Debug)]
pub enum ActivationError {
    Io(std::io::Error),
    NoSupportedIpmiLANVersions,
    GetChannelAuthenticationCapabilities(CommandError<()>),
    V1_5(V1_5ActivationError),
    RmcpError(RmcpUnwrapError),
}

impl From<V1_5ActivationError> for ActivationError {
    fn from(value: V1_5ActivationError) -> Self {
        Self::V1_5(value)
    }
}

impl From<std::io::Error> for ActivationError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

#[derive(Debug)]
pub struct Rmcp {
    unbound_state: RmcpWithState<Unbound>,
    active_state: Option<RmcpWithState<Active>>,
}

impl Rmcp {
    pub fn new<R>(remote: R, timeout: Duration) -> Result<Self, std::io::Error>
    where
        R: ToSocketAddrs + std::fmt::Debug,
    {
        let unbound_state = RmcpWithState::new(remote, timeout)?;

        Ok(Self {
            unbound_state,
            active_state: None,
        })
    }

    pub fn inactive_clone(&self) -> Self {
        Self {
            unbound_state: self.unbound_state.clone(),
            active_state: None,
        }
    }

    pub fn is_active(&self) -> bool {
        self.active_state.is_some()
    }

    pub fn activate(
        &mut self,
        username: Option<&str>,
        password: Option<&[u8]>,
    ) -> Result<(), ActivationError> {
        if let Some(_) = self.active_state.take() {
            // TODO: shut down currently active state.
            log::info!("De-activating RMCP connection for re-activation");
        }

        let inactive = self.unbound_state.bind()?;
        let activated = inactive.activate(username, password)?;
        self.active_state = Some(activated);
        Ok(())
    }
}

impl IpmiConnection for Rmcp {
    type SendError = RmcpError;

    type RecvError = RmcpError;

    type Error = RmcpError;

    fn send(&mut self, request: &mut crate::connection::Request) -> Result<(), Self::SendError> {
        let active = self.active_state.as_mut().ok_or(RmcpError::NotActive)?;
        active.send(request)
    }

    fn recv(&mut self) -> Result<crate::connection::Response, Self::RecvError> {
        let active = self.active_state.as_mut().ok_or(RmcpError::NotActive)?;
        active.recv()
    }

    fn send_recv(
        &mut self,
        request: &mut crate::connection::Request,
    ) -> Result<crate::connection::Response, Self::Error> {
        let active = self.active_state.as_mut().ok_or(RmcpError::NotActive)?;
        active.send_recv(request)
    }
}
