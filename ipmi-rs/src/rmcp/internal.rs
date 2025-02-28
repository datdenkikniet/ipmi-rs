// LE = least significant byte first = IPMI
// BE = most significant byte first = RMCP/ASF

use std::{
    io::ErrorKind,
    net::{SocketAddr, ToSocketAddrs, UdpSocket},
    time::Duration,
};

use crate::{
    app::auth::{GetChannelAuthenticationCapabilities, PrivilegeLevel},
    connection::{Channel, IpmiConnection, LogicalUnit, Response},
};

use super::{
    checksum::Checksum, v1_5::State as V1_5State, v2_0::State as V2_0State, ASFMessage,
    ASFMessageType, ActivationError, RmcpHeader, RmcpIpmiError, RmcpIpmiReceiveError, RmcpType,
};

#[derive(Debug, Clone)]
pub struct IpmbState {
    pub ipmb_sequence: u8,
    pub responder_addr: u8,
    pub requestor_addr: u8,
    pub requestor_lun: LogicalUnit,
}

impl Default for IpmbState {
    fn default() -> Self {
        Self {
            responder_addr: 0x20,
            requestor_addr: 0x81,
            requestor_lun: LogicalUnit::Zero,
            ipmb_sequence: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct Unbound {
    address: SocketAddr,
    timeout: Duration,
}

#[derive(Debug)]
pub struct Inactive {
    socket: UdpSocket,
}

#[derive(Debug)]
pub enum Active {
    V1_5(V1_5State),
    V2_0(V2_0State),
}

#[derive(Debug, Clone)]
pub(super) struct RmcpWithState<T>(T);

impl<T> RmcpWithState<T> {
    fn state(&self) -> &T {
        &self.0
    }

    fn state_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl RmcpWithState<Unbound> {
    pub fn new<R: ToSocketAddrs + core::fmt::Debug>(
        remote: R,
        timeout: Duration,
    ) -> std::io::Result<Self> {
        let address = remote.to_socket_addrs()?.next().ok_or_else(|| {
            std::io::Error::new(
                ErrorKind::NotFound,
                format!("Could not resolve any addresses for {remote:?}"),
            )
        })?;

        Ok(Self(Unbound { address, timeout }))
    }

    pub fn bind(&self) -> Result<RmcpWithState<Inactive>, std::io::Error> {
        let addr = &self.state().address;

        log::debug!("Binding socket...");
        let socket = UdpSocket::bind("[::]:0")?;
        socket.set_read_timeout(Some(self.state().timeout))?;

        log::debug!("Opening connection to {:?}", addr);
        socket.connect(addr)?;

        Ok(RmcpWithState(Inactive { socket }))
    }
}

impl RmcpWithState<Inactive> {
    pub fn activate(
        self,
        rmcp_plus: bool,
        username: Option<&str>,
        password: Option<&[u8]>,
    ) -> Result<RmcpWithState<Active>, ActivationError> {
        let message_tag = 0xC8;

        let ping_header = RmcpHeader::new_asf(0xFF);

        let ping = ASFMessage {
            message_tag,
            message_type: ASFMessageType::Ping,
        };

        log::debug!("Starting RMCP activation sequence");

        let socket = self.0.socket;

        // NOTE(unwrap): This cannot fail.
        let ping_bytes = ping_header.write_infallible(|buffer| {
            ping.write_data(buffer);
        });

        socket
            .send(&ping_bytes)
            .map_err(ActivationError::PingSend)?;

        let mut buf = [0u8; 1024];
        let received = socket
            .recv(&mut buf)
            .map_err(ActivationError::PongReceive)?;

        let (pong_header, pong_data) =
            RmcpHeader::from_bytes(&mut buf[..received]).map_err(|_| ActivationError::PongRead)?;

        let (supported_entities, _) = if pong_header.class().ty == RmcpType::Asf {
            let message = ASFMessage::from_bytes(pong_data).ok_or(ActivationError::PongRead)?;

            if message.message_tag != message_tag {
                return Err(ActivationError::PongRead);
            }

            if let ASFMessageType::Pong {
                supported_entities,
                supported_interactions,
                ..
            } = message.message_type
            {
                (supported_entities, supported_interactions)
            } else {
                return Err(ActivationError::PongRead);
            }
        } else {
            return Err(ActivationError::PongRead);
        };

        if !supported_entities.ipmi {
            return Err(ActivationError::IpmiNotSupported);
        }

        let new_state = V1_5State::new(socket);

        let mut ipmi = crate::Ipmi::new(new_state);

        log::debug!("Obtaining channel authentication capabilities");

        let privilege_level = PrivilegeLevel::Administrator;

        let authentication_caps = match ipmi.send_recv(GetChannelAuthenticationCapabilities::new(
            Channel::Current,
            privilege_level,
        )) {
            Ok(v) => v,
            Err(e) => return Err(ActivationError::GetChannelAuthenticationCapabilities(e)),
        };

        log::debug!("Authentication capabilities: {:?}", authentication_caps);

        if authentication_caps.ipmi2_connections_supported && rmcp_plus {
            let username = super::v2_0::Username::new(username.unwrap_or(""))
                .unwrap_or(super::v2_0::Username::new_empty());

            let socket = ipmi.release();

            let res = V2_0State::activate(
                socket,
                Some(privilege_level),
                &username,
                password.unwrap_or(&[]),
            )?;

            Ok(RmcpWithState(Active::V2_0(res)))
        } else if authentication_caps.ipmi15_connections_supported {
            let activated = ipmi.release().activate(
                &authentication_caps,
                privilege_level,
                username,
                password,
            )?;

            Ok(RmcpWithState(Active::V1_5(activated)))
        } else {
            Err(ActivationError::NoSupportedIpmiLANVersions)
        }
    }
}

impl IpmiConnection for RmcpWithState<Active> {
    type SendError = RmcpIpmiError;

    type RecvError = RmcpIpmiReceiveError;

    type Error = RmcpIpmiError;

    fn send(&mut self, request: &mut crate::connection::Request) -> Result<(), Self::SendError> {
        match self.state_mut() {
            Active::V1_5(state) => state.send(request)?,
            Active::V2_0(state) => state.send(request)?,
        }

        Ok(())
    }

    fn recv(&mut self) -> Result<Response, Self::RecvError> {
        match self.state_mut() {
            Active::V1_5(state) => state.recv(),
            Active::V2_0(state) => state.recv(),
        }
    }

    fn send_recv(
        &mut self,
        request: &mut crate::connection::Request,
    ) -> Result<Response, Self::Error> {
        match self.state_mut() {
            Active::V1_5(state) => state.send_recv(request),
            Active::V2_0(state) => state.send_recv(request),
        }
    }
}

pub fn validate_ipmb_checksums(data: &[u8]) -> bool {
    if data.len() < 4 {
        return false;
    }

    let first_checksum = Checksum::from_iter([data[0], data[1]]);

    if first_checksum != data[2] {
        return false;
    }

    let second_checksum = Checksum::from_iter(data[3..data.len() - 1].iter().copied());

    second_checksum == data[data.len() - 1]
}

// TODO: `ExactSizeIterator` to avoid/postpone allocation?
pub fn next_ipmb_message(
    request: &crate::connection::Request,
    ipmb_state: &mut IpmbState,
) -> Vec<u8> {
    let IpmbState {
        ipmb_sequence,
        responder_addr: rs_addr,
        requestor_addr,
        requestor_lun,
    } = ipmb_state;

    let data = request.data();

    let mut all_data = Vec::with_capacity(7 + data.len());

    let netfn_rslun: u8 = (request.netfn().request_value() << 2) | request.target().lun().value();
    let first_part = [*rs_addr, netfn_rslun];

    all_data.extend(first_part);
    all_data.push(Checksum::from_iter(first_part));

    let req_addr = *requestor_addr;

    let ipmb_sequence_val = *ipmb_sequence;
    *ipmb_sequence = ipmb_sequence.wrapping_add(1);

    let reqseq_lun = (ipmb_sequence_val << 2) | requestor_lun.value();
    let cmd = request.cmd();

    let second_start = [req_addr, reqseq_lun, cmd];
    let second_end = request.data().iter().copied();
    let second_part_chk = Checksum::from_iter(second_start.into_iter().chain(second_end.clone()));

    all_data.extend(second_start);
    all_data.extend(second_end);
    all_data.push(second_part_chk);

    all_data
}

#[test]
fn ipmb_message_test() {
    let data = next_ipmb_message(
        &crate::connection::Request::new(
            ipmi_rs_core::connection::NetFn::Transport,
            0x0B,
            vec![0x01, 0x02, 0x03],
            crate::connection::RequestTargetAddress::Bmc(crate::connection::LogicalUnit::One),
        ),
        &mut IpmbState::default(),
    );

    let expected = vec![0x20, 0x31, 0xAF, 0x81, 0x00, 0x0B, 0x01, 0x02, 0x03, 0x6E];

    assert_eq!(expected, data);
}
