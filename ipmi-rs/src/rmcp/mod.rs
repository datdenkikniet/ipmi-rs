use crate::{
    connection::{IpmiConnection, NotEnoughData},
    Ipmi, IpmiError,
};

use ipmi_rs_core::app::auth::{CloseSession, CloseSessionError};
use std::{net::ToSocketAddrs, time::Duration};

mod socket;

mod v1_5;
pub use v1_5::{
    ActivationError as V1_5ActivationError, ReadError as V1_5ReadError,
    WriteError as V1_5WriteError,
};

mod v2_0;
pub use v2_0::{
    ActivationError as V2_0ActivationError, ReadError as V2_0ReadError,
    WriteError as V2_0WriteError, *,
};

pub use ipmi_rs_core::app::auth::{
    AuthenticationAlgorithm, ConfidentialityAlgorithm, IntegrityAlgorithm,
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
    IpmbChecksumFailed,
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

type CommandError<T> = IpmiError<RmcpIpmiError, T>;

#[derive(Debug)]
pub enum ActivationError {
    BindSocket(std::io::Error),
    PingSend(std::io::Error),
    PongReceive(std::io::Error),
    PongRead,
    /// The contacted host does not support IPMI over RMCP.
    IpmiNotSupported,
    NoSupportedIpmiLANVersions,
    GetChannelAuthenticationCapabilities(CommandError<NotEnoughData>),
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

/// Errors that can occur while deactivating an RMCP connection.
#[derive(Debug)]
pub enum DeactivationError {
    /// There is no active session to close.
    NotActive,
    /// Closing the session failed. The connection is inactive afterwards
    /// regardless.
    CloseSession(CommandError<CloseSessionError>),
}

/// An RMCP connection to a BMC.
///
/// Sessions are not closed from `Drop`, because that would block in a
/// destructor. Call [`Rmcp::deactivate`] (or re-activate) to release the
/// session slot deterministically.
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

    /// Activate this RMCP connection with the provided username and password.
    ///
    /// If `rmcp_plus` is `true`, upgrade the connection to an RMCP+ connection
    /// if the remote host supports it.
    ///
    /// An active session is closed first. A failure to close it is logged at
    /// the `warn` level and does not stop activation. Because the close waits
    /// for the BMC's response, re-activation can take up to the connection's
    /// read timeout when the BMC does not answer.
    pub fn activate(
        &mut self,
        rmcp_plus: bool,
        username: Option<&str>,
        password: Option<&[u8]>,
    ) -> Result<(), ActivationError> {
        if self.is_active() {
            if let Err(error) = self.deactivate() {
                log::warn!("Failed to close the previous IPMI session: {error:?}");
            }
        }

        let inactive = self
            .unbound_state
            .bind()
            .map_err(ActivationError::BindSocket)?;

        let activated = inactive.activate(rmcp_plus, username, password)?;
        self.active_state = Some(activated);
        Ok(())
    }

    /// Close the active IPMI session so the BMC can release its slot.
    ///
    /// The connection is inactive afterwards, even when the BMC rejects the
    /// command or does not answer.
    pub fn deactivate(&mut self) -> Result<(), DeactivationError> {
        let Some(active) = self.active_state.take() else {
            return Err(DeactivationError::NotActive);
        };

        // `Active::V1_5` always carries a session ID after activation; stay
        // defensive rather than sending an ID the BMC never assigned.
        let Some(session_id) = active.session_id() else {
            log::debug!("Not closing the IPMI session: the active state has no session ID");
            return Ok(());
        };

        let mut ipmi = Ipmi::new(active);
        ipmi.send_recv(CloseSession { session_id })
            .map_err(DeactivationError::CloseSession)?;

        log::debug!("Closed IPMI session {:#010x}", session_id.get());

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deactivate_without_session_is_not_active() {
        // `Rmcp::new` only resolves the address; `deactivate` must return
        // before any socket is bound or any packet is sent.
        let mut rmcp =
            Rmcp::new("127.0.0.1:623", Duration::from_millis(10)).expect("resolve address");

        assert!(matches!(
            rmcp.deactivate(),
            Err(DeactivationError::NotActive)
        ));
        assert!(!rmcp.is_active());
    }
}
