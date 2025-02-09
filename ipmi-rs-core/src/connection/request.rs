use crate::connection::{LogicalUnit, NetFn};

use super::{Address, Channel, Message};

/// An IPMI request message.
pub struct Request {
    target: RequestTargetAddress,
    message: Message,
}

impl Request {
    /// Create a new IPMI request message.
    ///
    /// The netfn for `request` should be of the `request` variant, see [`Message::new_request`].
    // TODO: don't accept `Message` directly (could be malformed?)
    pub const fn new(request: Message, target: RequestTargetAddress) -> Self {
        Self {
            target,
            message: request,
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

    /// Get a shared reference to the data of the request (does not include netfn or command).

    pub fn data(&self) -> &[u8] {
        self.message.data()
    }

    /// Get a mutable reference to the data of the request (does not include netfn or command).
    pub fn data_mut(&mut self) -> &mut [u8] {
        self.message.data_mut()
    }

    /// Get the target for the request.
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
