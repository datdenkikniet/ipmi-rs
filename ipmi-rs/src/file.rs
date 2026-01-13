use std::fmt::{Display, Formatter};
use std::{ffi::c_int, io, os::fd::AsRawFd, time::Duration};

use ipmi_rs_core::connection::NetFn;
use nix::errno::Errno;
use nix::poll::{PollFd, PollFlags};

use crate::connection::{
    Address, IpmiConnection, Message, Request, RequestTargetAddress, Response,
};

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

impl IpmiMessage {
    fn log(&self, level: log::Level) {
        log::log!(level, "  NetFn      = 0x{:02X}", self.netfn);
        log::log!(level, "  Command    = 0x{:02X}", self.cmd);
        log::log!(level, "  Data len   = {}", self.data_len);
        if self.data_len > 0 {
            log::log!(level, "  Data       = {:02X?}", self.data());
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
    pub fn log(&self, level: log::Level) {
        log::log!(level, "  Message ID = 0x{:02X}", self.msg_id);
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

impl IpmiRecv {
    fn log(&self, level: log::Level) {
        log::log!(level, "  Type       = 0x{:02X}", self.recv_type);
        log::log!(level, "  Message ID = 0x{:02X}", self.msg_id);
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
            let message = Message::new_raw(netfn, cmd, value.message.data().to_vec());
            let response =
                Response::new(message, value.msg_id).ok_or(CreateResponseError::NotEnoughData)?;
            Ok(response)
        } else {
            Err(CreateResponseError::NotAResponse)
        }
    }
}

mod ioctl {
    const IPMI_IOC_MAGIC: u8 = b'i';

    use nix::{ioctl_read, ioctl_readwrite};

    use super::*;

    ioctl_readwrite!(ipmi_recv_msg_trunc, IPMI_IOC_MAGIC, 11, IpmiRecv);
    ioctl_read!(ipmi_send_request, IPMI_IOC_MAGIC, 13, IpmiRequest);
    ioctl_read!(ipmi_get_my_address, IPMI_IOC_MAGIC, 18, u32);
}

#[repr(C)]
enum IpmiAddr {
    SysIface(IpmiSysIfaceAddr),
    Ipmb(IpmiIpmbAddr),
}

impl IpmiAddr {
    fn ptr(&mut self) -> *mut u8 {
        match self {
            IpmiAddr::SysIface(ref mut bmc_addr) => std::ptr::addr_of_mut!(*bmc_addr) as *mut u8,
            IpmiAddr::Ipmb(ref mut ipmb_addr) => std::ptr::addr_of_mut!(*ipmb_addr) as *mut u8,
        }
    }
    fn size(&self) -> u32 {
        match self {
            IpmiAddr::SysIface(_) => core::mem::size_of::<IpmiSysIfaceAddr>() as u32,
            IpmiAddr::Ipmb(_) => core::mem::size_of::<IpmiIpmbAddr>() as u32,
        }
    }
}

impl From<RequestTargetAddress> for IpmiAddr {
    fn from(value: RequestTargetAddress) -> Self {
        match value {
            RequestTargetAddress::Bmc(lun) => {
                IpmiAddr::SysIface(IpmiSysIfaceAddr::bmc(lun.value()))
            }
            RequestTargetAddress::BmcOrIpmb(addr, channel, lun) => IpmiAddr::Ipmb(
                IpmiIpmbAddr::new(channel.value() as i16, addr.0, lun.value()),
            ),
        }
    }
}

impl Display for IpmiAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            IpmiAddr::SysIface(addr) => {
                write!(f, "System interface (LUN: {})", addr.lun)
            }
            IpmiAddr::Ipmb(addr) => {
                write!(
                    f,
                    "IPMB target (Channel: {}, Target: {}, LUN: {})",
                    addr.channel, addr.target_addr, addr.lun
                )
            }
        }
    }
}

#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub struct IpmiSysIfaceAddr {
    ty: i32,
    channel: i16,
    lun: u8,
}

#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub struct IpmiIpmbAddr {
    ty: i32,
    channel: i16,
    target_addr: u8,
    lun: u8,
}

impl IpmiIpmbAddr {
    const IPMI_IPMB_ADDR_TYPE: i32 = 0x01;

    pub const fn new(channel: i16, target_addr: u8, lun: u8) -> Self {
        Self {
            ty: Self::IPMI_IPMB_ADDR_TYPE,
            channel,
            target_addr,
            lun,
        }
    }
}

impl IpmiSysIfaceAddr {
    const IPMI_SYSTEM_INTERFACE_ADDR_TYPE: i32 = 0x0c;
    const IPMI_BMC_CHANNEL: i16 = 0xf;

    pub const fn bmc(lun: u8) -> Self {
        Self {
            ty: Self::IPMI_SYSTEM_INTERFACE_ADDR_TYPE,
            channel: Self::IPMI_BMC_CHANNEL,
            lun,
        }
    }
}

pub struct File {
    inner: std::fs::File,
    recv_timeout: Duration,
    seq: i64,
    my_addr: Address,
}

impl File {
    fn fd(&mut self) -> c_int {
        self.inner.as_raw_fd()
    }

    pub fn new(path: impl AsRef<std::path::Path>, recv_timeout: Duration) -> io::Result<Self> {
        let mut inner = std::fs::File::open(path)?;

        let my_addr = match Self::load_my_address_from_file(&mut inner) {
            Ok(addr) => addr,
            Err(e) => {
                log::warn!("Failed to get local address, defaulting to 0x20: {:?}", e);
                Address(0x20)
            }
        };

        Ok(Self {
            inner,
            recv_timeout,
            seq: -1,
            my_addr,
        })
    }

    fn load_my_address_from_file(file: &mut std::fs::File) -> io::Result<Address> {
        let mut my_addr: u32 = 8;
        unsafe { ioctl::ipmi_get_my_address(file.as_raw_fd(), std::ptr::addr_of_mut!(my_addr))? };
        if let Ok(addr) = u8::try_from(my_addr) {
            Ok(Address(addr))
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                format!("ipmi_get_my_address returned non-u8 address: {}", my_addr),
            ))
        }
    }
}

impl IpmiConnection for File {
    type SendError = io::Error;
    type RecvError = io::Error;
    type Error = io::Error;

    fn send(&mut self, request: &mut Request) -> io::Result<()> {
        let mut addr: IpmiAddr = match request.target() {
            RequestTargetAddress::BmcOrIpmb(a, _, lun) if a == self.my_addr => {
                RequestTargetAddress::Bmc(lun)
            }
            x => x,
        }
        .into();

        self.seq += 1;

        let netfn = request.netfn_raw();
        let cmd = request.cmd();
        let seq = self.seq;
        let data = request.data_mut();

        let data_len = data.len() as u16;
        let ptr = data.as_mut_ptr();

        log::debug!("Sending request (netfn: 0x{netfn:02X}, cmd: 0x{cmd:02X}) to {addr}");
        let ipmi_message = IpmiMessage {
            netfn,
            cmd,
            data_len,
            data: ptr,
        };
        let mut request = IpmiRequest {
            addr: addr.ptr(),
            addr_len: addr.size(),
            msg_id: seq,
            message: ipmi_message,
        };

        request.log(log::Level::Trace);

        // SAFETY: we send a mut pointer to an owned struct (`request`),
        // which has the correct layout for this IOCTL call.
        unsafe {
            ioctl::ipmi_send_request(self.fd(), std::ptr::addr_of_mut!(request))?;
        }

        // Ensure that data and bmc_addr live until _after_ the IOCTL completes.
        #[allow(clippy::drop_non_drop)]
        drop(request);
        #[allow(clippy::drop_non_drop)]
        drop(addr);

        Ok(())
    }

    fn recv(&mut self) -> io::Result<Response> {
        let start = std::time::Instant::now();

        let mut bmc_addr = IpmiSysIfaceAddr::bmc(0);

        let mut response_data = [0u8; 1024];

        let response_data_len = response_data.len() as u16;
        let response_data_ptr = response_data.as_mut_ptr();

        let mut recv = IpmiRecv {
            addr: std::ptr::addr_of_mut!(bmc_addr) as *mut u8,
            addr_len: core::mem::size_of::<IpmiSysIfaceAddr>() as u32,
            msg_id: 0,
            recv_type: 0,
            message: IpmiMessage {
                netfn: 0,
                cmd: 0,
                data_len: response_data_len,
                data: response_data_ptr,
            },
        };

        let mut polls = [PollFd::new(self.fd(), PollFlags::POLLIN)];
        let poll = nix::poll::poll(
            polls.as_mut_slice(),
            self.recv_timeout.as_millis().try_into().unwrap_or(i32::MAX),
        )?;

        if poll != 1 {
            log::warn!(
                "Failed to receive message after waiting for {} ms.",
                start.elapsed().as_millis(),
            );

            return Err(Errno::EAGAIN.into());
        }

        // SAFETY: we send a mut pointer to a fully owned struct (`recv`),
        // which has the correct layout for this IOCTL call.
        let ipmi_result =
            unsafe { ioctl::ipmi_recv_msg_trunc(self.fd(), std::ptr::addr_of_mut!(recv)) };

        let ipmi_result = match ipmi_result {
            Ok(_) => recv,
            Err(e) => {
                log::error!("Error occurred while reading from IPMI: {e:?}");
                return Err(e.into());
            }
        };

        // Ensure that response_data and bmc_addr live until _after_ the
        // IOCTL completes.
        #[allow(dropping_copy_types)]
        drop(response_data);
        #[allow(clippy::drop_non_drop)]
        drop(bmc_addr);

        log::debug!("Received response after {} ms", start.elapsed().as_millis());
        ipmi_result.log(log::Level::Trace);

        match Response::try_from(ipmi_result) {
            Ok(response) => {
                if response.seq() == self.seq {
                    Ok(response)
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!(
                            "Invalid sequence number on response. Expected {}, got {}",
                            self.seq,
                            response.seq()
                        ),
                    ))
                }
            }
            Err(e) => Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Error while creating response. {:?}", e),
            )),
        }
    }

    fn send_recv(&mut self, request: &mut Request) -> io::Result<Response> {
        self.send(request)?;

        self.recv()
    }
}
