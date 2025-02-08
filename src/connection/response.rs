use crate::NetFn;

use super::Message;

/// An IPMI response.
#[derive(Debug, Clone, PartialEq)]
pub struct Response {
    seq: i64,
    message: Message,
}

impl Response {
    /// Create a new IPMI request message.
    ///
    /// The netfn for `request` should be of the `request` variant, see [`Message::new_response`].
    pub fn new(message: Message, seq: i64) -> Option<Self> {
        if !message.data().is_empty() {
            Some(Self { message, seq })
        } else {
            None
        }
    }
    /// Get the netfn for the request.
    pub fn netfn(&self) -> NetFn {
        self.message.netfn()
    }

    /// Get the raw value of the netfn for the request.
    pub fn netfn_raw(&self) -> u8 {
        self.message.netfn_raw()
    }

    /// Get the command value for the request.
    pub fn cmd(&self) -> u8 {
        self.message.cmd
    }

    /// Get the sequence number for the response.
    pub fn seq(&self) -> i64 {
        self.seq
    }

    /// Get the completion code for the response.
    pub fn cc(&self) -> u8 {
        self.message.data[0]
    }

    /// Get a shared reference to the data of the request (does not include netfn or command).
    pub fn data(&self) -> &[u8] {
        &self.message.data[1..]
    }
}
