use crate::connection::{LogicalUnit, NetFn};

use super::{Address, Channel};

/// An IPMI request message.
#[derive(Clone)]
pub struct Request {
    target: RequestTargetAddress,
    netfn: u8,
    cmd: u8,
    data: Vec<u8>,
}

impl Request {
    /// Create a new IPMI request message.
    pub const fn new(netfn: NetFn, cmd: u8, data: Vec<u8>, target: RequestTargetAddress) -> Self {
        Self {
            target,
            netfn: netfn.request_value(),
            cmd,
            data,
        }
    }

    /// Create a new IPMI request message.
    pub const fn new_default_target(netfn: NetFn, cmd: u8, data: Vec<u8>) -> Self {
        let target = RequestTargetAddress::Bmc(LogicalUnit::Zero);
        Self::new(netfn, cmd, data, target)
    }

    /// Get the netfn for the request.
    pub fn netfn(&self) -> NetFn {
        self.netfn.into()
    }

    /// Get the raw value of the netfn for the request.
    pub fn netfn_raw(&self) -> u8 {
        self.netfn
    }

    /// Get the command value for the request.
    pub fn cmd(&self) -> u8 {
        self.cmd
    }

    /// Get a shared reference to the data of the request (does not include netfn or command).

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Get a mutable reference to the data of the request (does not include netfn or command).
    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// Get the intended target [`Address`] and [`Channel`] for this request.
    pub fn target(&self) -> RequestTargetAddress {
        self.target
    }
}

/// The target address of a request.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum RequestTargetAddress {
    /// A logical unit on the BMC (Board Management Controller).
    Bmc(LogicalUnit),
    /// An address on the BMC or IPMB.
    BmcOrIpmb(Address, Channel, LogicalUnit),
}

impl RequestTargetAddress {
    /// Get the logical unit for the target address.
    pub fn lun(&self) -> LogicalUnit {
        match self {
            RequestTargetAddress::Bmc(lun) | RequestTargetAddress::BmcOrIpmb(_, _, lun) => *lun,
        }
    }
}
