use crate::connection::{LogicalUnit, NetFn};

pub struct Request {
    net_fn: NetFn,
    lun: LogicalUnit,
    seq: i64,
    data: Vec<u8>,
}

impl Request {
    pub const fn new(net_fn: NetFn, lun: LogicalUnit, seq: i64, data: Vec<u8>) -> Self {
        Self {
            net_fn,
            lun,
            seq,
            data,
        }
    }

    pub fn netfn(&self) -> &NetFn {
        &self.net_fn
    }

    pub fn seq(&self) -> i64 {
        self.seq
    }

    pub fn lun(&self) -> LogicalUnit {
        self.lun
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn length(&self) -> u8 {
        1 + 1 + 1 + 1 + self.data.len() as u8
    }
}
