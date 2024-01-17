use crate::connection::{LogicalUnit, NetFn};

use super::Message;

pub struct Request {
    lun: LogicalUnit,
    message: Message,
    address_and_channel: Option<(u8, u8)>,
}

impl Request {
    pub const fn new(
        request: Message,
        lun: LogicalUnit,
        address_and_channel: Option<(u8, u8)>,
    ) -> Self {
        Self {
            lun,
            message: request,
            address_and_channel,
        }
    }

    pub fn netfn(&self) -> NetFn {
        self.message.netfn()
    }

    pub fn netfn_raw(&self) -> u8 {
        self.message.netfn_raw()
    }

    pub fn lun(&self) -> LogicalUnit {
        self.lun
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

    pub fn bridge_target_address_and_channel(&self, my_addr: u8) -> Option<(u8, u8)> {
        match self.address_and_channel {
            Some((addr, channel)) => {
                if addr != my_addr {
                    Some((addr, channel))
                } else {
                    None
                }
            }
            None => None,
        }
    }
}
