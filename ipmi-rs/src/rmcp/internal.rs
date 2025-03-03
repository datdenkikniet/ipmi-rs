// LE = least significant byte first = IPMI
// BE = most significant byte first = RMCP/ASF

use std::{
    io::ErrorKind,
    net::{SocketAddr, ToSocketAddrs, UdpSocket},
    time::Duration,
};

use ipmi_rs_core::{app::auth::ChannelAuthenticationCapabilities, connection::IpmiCommand};

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
    V1_5 { state: V1_5State, socket: UdpSocket },
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
    fn ping_pong(socket: &mut UdpSocket) -> Result<(), ActivationError> {
        let mut recv_buf = [0u8; 1024];

        let message_tag = 0xC8;

        let ping_header = RmcpHeader::new_asf(0xFF);

        let ping = ASFMessage {
            message_tag,
            message_type: ASFMessageType::Ping,
        };

        // NOTE(unwrap): This cannot fail.
        let ping_bytes = ping_header.write_infallible(|buffer| {
            ping.write_data(buffer);
        });

        socket
            .send(&ping_bytes)
            .map_err(ActivationError::PingSend)?;

        let received = socket
            .recv(&mut recv_buf)
            .map_err(ActivationError::PongReceive)?;

        let (pong_header, pong_data) = RmcpHeader::from_bytes(&mut recv_buf[..received])
            .map_err(|_| ActivationError::PongRead)?;

        let (supported_entities, _) = if pong_header.class().ty == RmcpType::Asf {
            let message = ASFMessage::from_bytes(pong_data).ok_or(ActivationError::PongParse)?;

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

        Ok(())
    }

    fn activate_shake(
        socket: &mut UdpSocket,
        username: Option<&str>,
        password: Option<&[u8]>,
        caps: &ChannelAuthenticationCapabilities,
        privilege_level: PrivilegeLevel,
    ) -> Result<V1_5State, ActivationError> {
        use ActivationError::*;

        let mut recv_buffer = [0u8; 1024];
        let state = V1_5State::new();

        let (state, challenge_request) = state.activate(username, password)?;

        socket.send(&challenge_request).map_err(Io)?;
        let recv_len = socket.recv(&mut recv_buffer).map_err(Io)?;

        let (state, activation) =
            state.recv_session_challenge(&mut recv_buffer[..recv_len], caps, privilege_level)?;

        socket.send(&activation).map_err(Io)?;
        let recv_len = socket.recv(&mut recv_buffer).map_err(Io)?;

        let active = state.recv_session_info(&mut recv_buffer[..recv_len])?;

        Ok(active)
    }

    fn get_channel_auth_caps(
        socket: &mut UdpSocket,
        level: PrivilegeLevel,
    ) -> Result<ChannelAuthenticationCapabilities, ActivationError> {
        use ActivationError::*;

        let mut recv_buf = [0u8; 1024];
        log::debug!("Obtaining channel authentication capabilities");

        let mut new_state = V1_5State::new();

        let get_command = GetChannelAuthenticationCapabilities::new(Channel::Current, level);

        let data = new_state
            .send(get_command)
            .expect("Writing messages on newly initialized state is always valid");

        socket.send(&data).map_err(SendGetChannelAuthCaps)?;

        let data_len = socket.recv(&mut recv_buf).map_err(RecvChannelAuthCaps)?;

        let response = new_state
            .recv(&mut recv_buf[..data_len])
            .map_err(ReadChannelAuthCaps)?;

        let authentication_caps =
            GetChannelAuthenticationCapabilities::parse_success_response(response.data())
                .map_err(ParseChannelAuthCaps)?;

        log::debug!("Authentication capabilities: {:?}", authentication_caps);

        Ok(authentication_caps)
    }

    pub fn activate(
        self,
        rmcp_plus: bool,
        username: Option<&str>,
        password: Option<&[u8]>,
    ) -> Result<RmcpWithState<Active>, ActivationError> {
        let privilege_level = PrivilegeLevel::Administrator;

        log::debug!("Starting RMCP activation sequence");

        let mut socket = self.0.socket;

        Self::ping_pong(&mut socket)?;
        let authentication_caps = Self::get_channel_auth_caps(&mut socket, privilege_level)?;

        if authentication_caps.ipmi2_connections_supported && rmcp_plus {
            // let username = super::v2_0::Username::new(username.unwrap_or(""))
            //     .unwrap_or(super::v2_0::Username::new_empty());

            // let socket = ipmi.release();

            // let res = V2_0State::activate(
            //     socket,
            //     Some(privilege_level),
            //     &username,
            //     password.unwrap_or(&[]),
            // )?;

            // Ok(RmcpWithState(Active::V2_0(res)))
            todo!()
        } else if authentication_caps.ipmi15_connections_supported {
            let activated = Self::activate_shake(
                &mut socket,
                username,
                password,
                &authentication_caps,
                privilege_level,
            )?;

            Ok(RmcpWithState(Active::V1_5 {
                state: activated,
                socket,
            }))
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
            Active::V1_5 { state, socket } => {
                let cloned = request.clone();
                let out_data = state
                    .send(cloned)
                    .map_err(|e| RmcpIpmiError::Send(super::RmcpIpmiSendError::V1_5(e)))?;
                socket.send(&out_data).map_err(RmcpIpmiError::Io)?;
            }
            Active::V2_0(state) => state.send(request)?,
        }

        Ok(())
    }

    fn recv(&mut self) -> Result<Response, Self::RecvError> {
        match self.state_mut() {
            Active::V1_5 { state, socket } => {
                let mut recv_buffer = [0u8; 1024];
                let data_len = socket
                    .recv(&mut recv_buffer)
                    .map_err(RmcpIpmiReceiveError::Io)?;
                let out = state.recv(&mut recv_buffer[..data_len])?;
                Ok(out)
            }
            Active::V2_0(state) => state.recv(),
        }
    }

    fn send_recv(
        &mut self,
        request: &mut crate::connection::Request,
    ) -> Result<Response, Self::Error> {
        self.send(request)?;
        let res = self.recv()?;
        Ok(res)
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
