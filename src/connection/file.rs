use std::{
    ffi::c_int,
    io,
    os::fd::AsRawFd,
    time::{Duration, Instant},
};

use crate::{
    connection::{Request, Response},
    fmt::{LogOutput, Loggable},
    NetFn,
};

use super::Message;

#[repr(C)]
#[derive(Debug)]
pub struct IpmiMessage {
    netfn: u8,
    cmd: u8,
    data_len: u16,
    data: *mut u8,
}

impl IpmiMessage {
    fn data(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.data, self.data_len as usize) }
    }
}

impl Loggable for IpmiMessage {
    fn log(&self, level: LogOutput) {
        use crate::log;
        log!(level, "  NetFn      = 0x{:02X}", self.netfn);
        log!(level, "  Command    = 0x{:02X}", self.cmd);
        log!(level, "  Data len   = {}", self.data_len);
        if self.data_len > 0 {
            log!(level, "  Data       = {:02X?}", self.data());
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct IpmiRequest {
    addr: *mut u8,
    addr_len: u32,
    msg_id: i64,
    message: IpmiMessage,
}

impl IpmiRequest {
    pub fn log(&self, level: LogOutput) {
        crate::log!(level, "  Message ID = 0x{:02X}", self.msg_id);
        self.message.log(level)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct IpmiRecv {
    recv_type: i32,
    addr: *mut u8,
    addr_len: u32,
    msg_id: i64,
    message: IpmiMessage,
}

impl Loggable for IpmiRecv {
    fn log(&self, level: LogOutput) {
        crate::log!(level, "  Type       = 0x{:02X}", self.recv_type);
        crate::log!(level, "  Message ID = 0x{:02X}", self.msg_id);
        self.message.log(level)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum CreateResponseError {
    NotAResponse,
    NotEnoughData,
    InvalidCmd,
}

impl TryFrom<IpmiRecv> for Response {
    type Error = CreateResponseError;

    fn try_from(value: IpmiRecv) -> Result<Self, Self::Error> {
        let (netfn, cmd) = (value.message.netfn, value.message.cmd);

        let netfn_parsed = NetFn::from(netfn);

        if netfn_parsed.response_value() == netfn {
            let message = Message::new(netfn_parsed, cmd, value.message.data().to_vec());
            let response =
                Response::new(message, value.msg_id).ok_or(CreateResponseError::NotEnoughData)?;
            Ok(response)
        } else {
            Err(CreateResponseError::NotAResponse)
        }
    }
}

mod ioctl {
    const IPMI_IOC_MAGIC: u8 = 'i' as u8;

    use nix::{ioctl_read, ioctl_readwrite};

    use super::*;

    ioctl_readwrite!(ipmi_recv_msg_trunc, IPMI_IOC_MAGIC, 11, IpmiRecv);
    ioctl_read!(ipmi_send_request, IPMI_IOC_MAGIC, 13, IpmiRequest);
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IpmiSysIfaceAddr {
    ty: i32,
    channel: i16,
}

impl IpmiSysIfaceAddr {
    const IPMI_SYSTEM_INTERFACE_ADDR_TYPE: i32 = 0x0c;
    const IPMI_BMC_CHANNEL: i16 = 0xf;

    pub const fn bmc() -> Self {
        Self {
            ty: Self::IPMI_SYSTEM_INTERFACE_ADDR_TYPE,
            channel: Self::IPMI_BMC_CHANNEL,
        }
    }
}

pub struct File {
    inner: std::fs::File,
    recv_timeout: Duration,
}

impl File {
    fn fd(&mut self) -> c_int {
        self.inner.as_raw_fd()
    }

    pub fn new(path: impl AsRef<std::path::Path>, recv_timeout: Duration) -> io::Result<Self> {
        Ok(Self {
            inner: std::fs::File::open(path)?,
            recv_timeout,
        })
    }
}

impl super::IpmiConnection for File {
    type SendError = io::Error;
    type RecvError = io::Error;
    type Error = io::Error;

    fn send(&mut self, request: &mut Request) -> io::Result<()> {
        let bmc_addr = &mut IpmiSysIfaceAddr::bmc();

        let netfn = request.netfn().request_value();
        let cmd = request.cmd();
        let seq = request.seq();
        let data = request.data_mut();

        let data_len = data.len() as u16;
        let ptr = data.as_mut_ptr();

        let mut request = IpmiRequest {
            addr: bmc_addr as *mut _ as *mut u8,
            addr_len: core::mem::size_of_val(bmc_addr) as u32,
            msg_id: seq,
            message: IpmiMessage {
                netfn,
                cmd,
                data_len,
                data: ptr,
            },
        };

        log::debug!("Sending request");
        request.log(log::Level::Trace.into());

        // SAFETY: we send a mut pointer to an owned struct (`request`),
        // which has the correct layout for this IOCTL call.
        unsafe {
            ioctl::ipmi_send_request(self.fd(), &mut request as *mut _)?;
        }

        // Ensure that data and bmc_addr live until _after_ the IOCTL completes.
        drop((data, bmc_addr));

        Ok(())
    }

    fn recv(&mut self) -> io::Result<Response> {
        let start = std::time::Instant::now();

        let bmc_addr = &mut IpmiSysIfaceAddr::bmc();

        let mut response_data = [0u8; 1024];

        let response_data_len = response_data.len() as u16;
        let response_data_ptr = response_data.as_mut_ptr();

        let mut recv = IpmiRecv {
            addr: bmc_addr as *mut _ as *mut u8,
            addr_len: core::mem::size_of_val(bmc_addr) as u32,
            msg_id: 0,
            recv_type: 0,
            message: IpmiMessage {
                netfn: 0,
                cmd: 0,
                data_len: response_data_len,
                data: response_data_ptr,
            },
        };

        let start_time = Instant::now();

        let ipmi_result = loop {
            // SAFETY: we send a mut pointer to a fully owned struct (`recv`),
            // which has the correct layout for this IOCTL call.
            let ioctl_result =
                unsafe { ioctl::ipmi_recv_msg_trunc(self.fd(), &mut recv as *mut _) };

            match ioctl_result {
                Ok(_) => break Ok(recv),
                Err(e) => {
                    if Instant::now().duration_since(start_time) > self.recv_timeout {
                        break Err(e);
                    } else {
                        continue;
                    }
                }
            }
        };

        // Ensure that response_data and bmc_addr live until _after_ the
        // IOCTL completes.
        drop((response_data, bmc_addr));

        let end = std::time::Instant::now();
        let duration = (end - start).as_millis() as u32;

        let result = match ipmi_result {
            Ok(recv) => {
                log::debug!("Received response after {} ms", duration);
                recv.log(log::Level::Trace.into());

                recv.try_into().map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::Other,
                        format!("Error while creating response. {:?}", e),
                    )
                })
            }
            Err(e) => {
                log::warn!(
                    "Failed to receive message after waiting for {} ms. {:?}",
                    e,
                    duration
                );
                Err(e.into())
            }
        };

        result
    }

    fn send_recv(&mut self, request: &mut Request) -> io::Result<Response> {
        self.send(request)?;
        self.recv()
    }
}
