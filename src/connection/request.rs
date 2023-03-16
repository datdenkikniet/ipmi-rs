use crate::connection::{LogicalUnit, NetFn};

pub struct Request {
    net_fn: NetFn,
    lun: LogicalUnit,
    seq: i64,
}

impl Request {
    pub const fn new(net_fn: NetFn, lun: LogicalUnit, seq: i64) -> Self {
        Self { net_fn, lun, seq }
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
}
