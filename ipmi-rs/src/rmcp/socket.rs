use std::net::UdpSocket;

use super::RmcpIpmiReceiveError;

#[derive(Debug)]
pub struct RmcpIpmiSocket {
    socket: UdpSocket,
    buffer: [u8; 1024],
}

impl RmcpIpmiSocket {
    pub fn new(socket: UdpSocket) -> Self {
        Self {
            socket,
            buffer: [0u8; 1024],
        }
    }

    pub fn release(self) -> UdpSocket {
        self.socket
    }

    pub fn recv(&mut self) -> Result<&mut [u8], RmcpIpmiReceiveError> {
        let received = self
            .socket
            .recv(&mut self.buffer)
            .map_err(super::RecvError::Io)?;
        super::read_rmcp_data(&mut self.buffer[..received])
    }

    pub fn send<F, E>(&mut self, data: F) -> Result<(), E>
    where
        F: FnMut(&mut Vec<u8>) -> Result<(), E>,
        E: From<std::io::Error>,
    {
        let data = super::write_ipmi_data(data)?;
        self.socket.send(&data).map(|_| ()).map_err(From::from)
    }
}
