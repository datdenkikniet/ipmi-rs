use crate::{connection::IpmiConnection, IpmiCommandError};
use std::{net::ToSocketAddrs, time::Duration};

mod socket;

mod v1_5;
pub use v1_5::{
    ActivationError as V1_5ActivationError, ReadError as V1_5ReadError,
    WriteError as V1_5WriteError,
};

mod v2_0;
pub use v2_0::{
    ActivationError as V2_0ActivationError, AuthenticationAlgorithm, ConfidentialityAlgorithm,
    IntegrityAlgorithm, ReadError as V2_0ReadError, WriteError as V2_0WriteError, *,
};

mod checksum;

mod header;
pub(crate) use header::*;

mod asf;
pub(crate) use asf::*;

mod internal;
use internal::{Active, RmcpWithState, Unbound};

#[derive(Debug)]
pub enum RmcpIpmiReceiveError {
    Io(std::io::Error),
    RmcpHeader(RmcpHeaderError),
    Session(UnwrapSessionError),
    NotIpmi,
    NotEnoughData,
    EmptyMessage,
}

#[derive(Debug)]
pub enum RmcpIpmiSendError {
    V1_5(V1_5WriteError),
    V2_0(V2_0WriteError),
}

impl From<V1_5WriteError> for RmcpIpmiSendError {
    fn from(value: V1_5WriteError) -> Self {
        Self::V1_5(value)
    }
}

impl From<V2_0WriteError> for RmcpIpmiSendError {
    fn from(value: V2_0WriteError) -> Self {
        Self::V2_0(value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnwrapSessionError {
    V1_5(V1_5ReadError),
    V2_0(V2_0ReadError),
}

impl From<V1_5ReadError> for UnwrapSessionError {
    fn from(value: V1_5ReadError) -> Self {
        Self::V1_5(value)
    }
}

impl From<V2_0ReadError> for UnwrapSessionError {
    fn from(value: V2_0ReadError) -> Self {
        Self::V2_0(value)
    }
}

#[derive(Debug)]
pub enum RmcpIpmiError {
    NotActive,
    Receive(RmcpIpmiReceiveError),
    Send(RmcpIpmiSendError),
}

impl From<RmcpIpmiReceiveError> for RmcpIpmiError {
    fn from(value: RmcpIpmiReceiveError) -> Self {
        Self::Receive(value)
    }
}

impl From<RmcpIpmiSendError> for RmcpIpmiError {
    fn from(value: RmcpIpmiSendError) -> Self {
        Self::Send(value)
    }
}

type CommandError<T> = IpmiCommandError<RmcpIpmiError, T>;

#[derive(Debug)]
pub enum ActivationError {
    BindSocket(std::io::Error),
    PingSend(std::io::Error),
    PongReceive(std::io::Error),
    PongRead,
    /// The contacted host does not support IPMI over RMCP.
    IpmiNotSupported,
    NoSupportedIpmiLANVersions,
    GetChannelAuthenticationCapabilities(CommandError<()>),
    V1_5(V1_5ActivationError),
    V2_0(V2_0ActivationError),
    RmcpError(RmcpHeaderError),
}

impl From<V1_5ActivationError> for ActivationError {
    fn from(value: V1_5ActivationError) -> Self {
        Self::V1_5(value)
    }
}

impl From<V2_0ActivationError> for ActivationError {
    fn from(value: V2_0ActivationError) -> Self {
        Self::V2_0(value)
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
        if self.active_state.take().is_some() {
            // TODO: shut down currently active state.
            log::info!("De-activating RMCP connection for re-activation");
        }

        let inactive = self
            .unbound_state
            .bind()
            .map_err(ActivationError::BindSocket)?;

        let activated = inactive.activate(username, password)?;
        self.active_state = Some(activated);
        Ok(())
    }
}

impl IpmiConnection for Rmcp {
    type SendError = RmcpIpmiError;

    type RecvError = RmcpIpmiError;

    type Error = RmcpIpmiError;

    fn send(&mut self, request: &mut crate::connection::Request) -> Result<(), Self::SendError> {
        let active = self.active_state.as_mut().ok_or(RmcpIpmiError::NotActive)?;
        active.send(request)
    }

    fn recv(&mut self) -> Result<crate::connection::Response, Self::RecvError> {
        let active = self.active_state.as_mut().ok_or(RmcpIpmiError::NotActive)?;
        active.recv().map_err(RmcpIpmiError::Receive)
    }

    fn send_recv(
        &mut self,
        request: &mut crate::connection::Request,
    ) -> Result<crate::connection::Response, Self::Error> {
        let active = self.active_state.as_mut().ok_or(RmcpIpmiError::NotActive)?;
        active.send_recv(request)
    }
}
