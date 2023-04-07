use crate::NetFn;

use super::Message;

#[derive(Debug, Clone, PartialEq)]
pub struct Response {
    seq: i64,
    message: Message,
}

impl Response {
    pub fn new(message: Message, seq: i64) -> Option<Self> {
        if !message.data().is_empty() {
            Some(Self { message, seq })
        } else {
            None
        }
    }

    pub fn netfn(&self) -> NetFn {
        self.message.netfn
    }

    pub fn cmd(&self) -> u8 {
        self.message.cmd
    }

    pub fn seq(&self) -> i64 {
        self.seq
    }

    pub fn cc(&self) -> u8 {
        self.message.data[0]
    }

    pub fn data(&self) -> &[u8] {
        &self.message.data[1..]
    }
}
