use crate::connection::{Address, LogicalUnit, NetFn, RequestTargetAddress};

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

    pub fn lun(&self) -> LogicalUnit {
        match self.target {
            RequestTargetAddress::Bmc(lun) => lun,
            RequestTargetAddress::BmcOrIpmb(_, _, lun) => lun,
        }
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

    pub fn bridge_target_address_and_channel(&self, my_addr: Address) -> RequestTargetAddress {
        match self.target {
            RequestTargetAddress::BmcOrIpmb(a, _, lun) if a == my_addr => {
                RequestTargetAddress::Bmc(lun)
            }
            x => x,
        }
    }
}
