use crate::connection::{LogicalUnit, NetFn};

use super::Message;

pub struct Request {
    lun: LogicalUnit,
    seq: i64,
    message: Message,
}

impl Request {
    pub const fn new(request: Message, lun: LogicalUnit, seq: i64) -> Self {
        Self {
            lun,
            seq,
            message: request,
        }
    }

    pub fn netfn(&self) -> NetFn {
        self.message.netfn()
    }

    pub fn seq(&self) -> i64 {
        self.seq
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
