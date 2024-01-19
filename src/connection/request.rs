use crate::connection::{LogicalUnit, NetFn};

use super::Message;

pub struct Request {
    target: RequestTargetAddress,
    message: Message,
}

impl Request {
    pub const fn new(request: Message, target: RequestTargetAddress) -> Self {
        Self {
            target,
            message: request,
        }
    }

    pub fn netfn(&self) -> NetFn {
        self.message.netfn()
    }

    pub fn netfn_raw(&self) -> u8 {
        self.message.netfn_raw()
    }

    pub fn cmd(&self) -> u8 {
        self.message.cmd
    }

    pub fn data(&self) -> &[u8] {
        self.message.data()
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        self.message.data_mut()
    }

    pub fn target(&self) -> RequestTargetAddress {
        self.target
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Address(pub u8);
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Channel(pub u8);

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum RequestTargetAddress {
    Bmc(LogicalUnit),
    BmcOrIpmb(Address, Channel, LogicalUnit),
}

impl RequestTargetAddress {
    pub fn lun(&self) -> LogicalUnit {
        match self {
            RequestTargetAddress::Bmc(lun) | RequestTargetAddress::BmcOrIpmb(_, _, lun) => *lun,
        }
    }
}
