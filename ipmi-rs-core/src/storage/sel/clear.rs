//! Clear SEL Command
//!
//! Reference: IPMI 2.0 Specification, Section 31.9 "Clear SEL Command"

use core::num::NonZeroU16;

use crate::connection::{EncodeIpmiCommand, IpmiCommand, NetFn, NotEnoughData};

/// Action to perform when clearing the SEL.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClearSelAction {
    /// Initiate erase (0xAA)
    InitiateErase,
    /// Get erasure status (0x00)
    GetStatus,
}

/// Erasure progress status.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ErasureProgress {
    /// Erasure in progress
    InProgress,
    /// Erase completed
    Completed,
}

/// Clear SEL command.
///
/// This command is used to erase all records in the SEL. The command requires
/// a valid Reservation ID obtained from the Reserve SEL command unless the
/// implementation does not support SEL reservation. In that case, the
/// Reservation ID should be set to 0x0000.
///
/// The clearing operation is a two-step process:
/// 1. Send ClearSel with `InitiateErase` action to start the erase
/// 2. Optionally poll with `GetStatus` action to check completion
///
/// Reference: IPMI 2.0 Specification, Section 31.9, Table 31-9
pub struct ClearSel {
    reservation_id: Option<NonZeroU16>,
    action: ClearSelAction,
}

impl ClearSel {
    /// Create a new ClearSel command to initiate erasure.
    pub fn initiate(reservation_id: Option<NonZeroU16>) -> Self {
        Self {
            reservation_id,
            action: ClearSelAction::InitiateErase,
        }
    }

    /// Create a new ClearSel command to get erasure status.
    pub fn get_status(reservation_id: Option<NonZeroU16>) -> Self {
        Self {
            reservation_id,
            action: ClearSelAction::GetStatus,
        }
    }
}

impl IpmiCommand for ClearSel {
    type Output = ErasureProgress;
    type Error = NotEnoughData;

    /// Parse the response which contains the erasure progress.
    ///
    /// Response data format (IPMI 2.0 Spec, Table 31-9):
    /// - Byte 0: Erasure progress
    ///   - \[3:0\]: 0h = erasure in progress, 1h = erase completed
    fn parse_success_response(data: &[u8]) -> Result<Self::Output, Self::Error> {
        if data.is_empty() {
            return Err(NotEnoughData);
        }

        let progress = data[0] & 0x0F;
        Ok(if progress == 0x01 {
            ErasureProgress::Completed
        } else {
            ErasureProgress::InProgress
        })
    }
}

impl EncodeIpmiCommand for ClearSel {
    const NETFN: NetFn = NetFn::Storage;
    const CMD: u8 = 0x47;

    fn request_data_len(&self) -> usize {
        6
    }

    // Request format (IPMI 2.0 Spec, Table 31-9):
    // - Byte 0-1: Reservation ID, LS byte first
    // - Byte 2: 'C' (0x43)
    // - Byte 3: 'L' (0x4C)
    // - Byte 4: 'R' (0x52)
    // - Byte 5: Action
    //   - 0xAA = initiate erase
    //   - 0x00 = get erasure status
    fn write_request_data(&self, data: &mut [u8]) {
        data[0..2].copy_from_slice(
            &self
                .reservation_id
                .map_or(0, core::num::NonZeroU16::get)
                .to_le_bytes(),
        );
        // NetFn: Storage (0x0A), Cmd: 0x47
        data[2] = 0x43; // 'C'
        data[3] = 0x4C; // 'L'
        data[4] = 0x52; // 'R'
        data[5] = match self.action {
            ClearSelAction::InitiateErase => 0xaa,
            ClearSelAction::GetStatus => 0x00,
        };
    }
}
