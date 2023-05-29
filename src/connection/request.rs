use crate::connection::{LogicalUnit, NetFn};

use super::Message;

pub struct Request {
    lun: LogicalUnit,
    message: Message,
}

impl Request {
    pub const fn new(request: Message, lun: LogicalUnit) -> Self {
        Self {
            lun,
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
}
