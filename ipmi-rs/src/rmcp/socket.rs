use std::net::UdpSocket;

use super::{RmcpHeader, RmcpIpmiReceiveError, RmcpType};

type RecvError = RmcpIpmiReceiveError;

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
        let received = self.socket.recv(&mut self.buffer).map_err(RecvError::Io)?;

        let data = &mut self.buffer[..received];
        let (header, data) = RmcpHeader::from_bytes(data).map_err(RecvError::RmcpHeader)?;

        if header.class().ty != RmcpType::Ipmi {
            return Err(RecvError::NotIpmi);
        }

        Ok(data)
    }

    pub fn send<F, E>(&mut self, data: F) -> Result<(), E>
    where
        F: FnMut(&mut Vec<u8>) -> Result<(), E>,
        E: From<std::io::Error>,
    {
        let header = RmcpHeader::new_ipmi();

        let data = header.write(data)?;

        self.socket.send(&data).map(|_| ()).map_err(From::from)
    }
}
