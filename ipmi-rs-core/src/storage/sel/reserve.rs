//! Reserve SEL Command
//!
//! Reference: IPMI 2.0 Specification, Section 31.4 "Reserve SEL Command"

use std::num::NonZeroU16;

use crate::connection::{IpmiCommand, Message, NetFn, NotEnoughData};

/// Reserve SEL command.
///
/// This command is used to set the present 'owner' of the SEL, and is used
/// to provide a mechanism to prevent SEL record deletion from being corrupted
/// when multiple parties are accessing the SEL.
///
/// A reservation ID is required before clearing the SEL.
///
/// Reference: IPMI 2.0 Specification, Section 31.4, Table 31-4
pub struct ReserveSel;

impl IpmiCommand for ReserveSel {
    type Output = NonZeroU16;
    type Error = NotEnoughData;

    /// Parse the response which contains the Reservation ID.
    ///
    /// Response data format (IPMI 2.0 Spec, Table 31-4):
    /// - Byte 0: Reservation ID, LS Byte
    /// - Byte 1: Reservation ID, MS Byte
    fn parse_success_response(data: &[u8]) -> Result<Self::Output, Self::Error> {
        if data.len() < 2 {
            return Err(NotEnoughData);
        }

        let reservation_id = u16::from_le_bytes([data[0], data[1]]);
        // Reservation ID of 0 is not valid per spec
        NonZeroU16::new(reservation_id).ok_or(NotEnoughData)
    }
}

impl From<ReserveSel> for Message {
    /// Build the request message.
    ///
    /// Request format (IPMI 2.0 Spec, Table 31-4):
    /// - No request data
    fn from(_: ReserveSel) -> Self {
        // NetFn: Storage (0x0A), Cmd: 0x42
        Message::new_request(NetFn::Storage, 0x42, Vec::new())
    }
}
