use core::num::NonZeroU16;

use nonmax::NonMaxU8;

#[cfg(feature = "alloc")]
use crate::connection::IpmiCommand;
use crate::connection::{EncodeIpmiCommand, NetFn};

use super::RecordId;
#[cfg(feature = "alloc")]
use super::{Record, RecordParseError};

/// Get a device SDR.
///
/// This command must be used in accordance with the IPMI spec, i.e.
/// all SDRs must be obtained sequentially. It is recommended that you use
/// the function `Ipmi::sdrs` in the `ipmi-rs` function for this.
#[derive(Debug, Clone, Copy)]
pub struct GetDeviceSdr {
    reservation_id: Option<NonZeroU16>,
    record_id: RecordId,
    offset: u8,
    bytes_to_read: Option<NonMaxU8>,
}

impl GetDeviceSdr {
    pub fn new(reservation_id: Option<NonZeroU16>, record_id: RecordId) -> Self {
        Self {
            reservation_id,
            record_id,
            // Always read all bytes
            offset: 0,
            bytes_to_read: None,
        }
    }
}

impl EncodeIpmiCommand for GetDeviceSdr {
    const NETFN: NetFn = NetFn::Storage;
    const CMD: u8 = 0x23;

    fn request_data_len(&self) -> usize {
        6
    }

    fn write_request_data(&self, data: &mut [u8]) {
        data[0..2].copy_from_slice(
            &self
                .reservation_id
                .map(NonZeroU16::get)
                .unwrap_or(0)
                .to_le_bytes(),
        );
        data[2..4].copy_from_slice(&self.record_id.value().to_le_bytes());
        data[4] = self.offset;
        data[5] = self.bytes_to_read.map(|value| value.get()).unwrap_or(0xff);
    }
}

#[cfg(feature = "alloc")]
impl IpmiCommand for GetDeviceSdr {
    type Output = RecordInfo;

    type Error = (RecordParseError, Option<RecordId>);

    fn parse_success_response(data: &[u8]) -> Result<Self::Output, Self::Error> {
        if data.len() < 9 {
            return Err((RecordParseError::NotEnoughData, None));
        }

        let next_id = RecordId::new_raw(u16::from_le_bytes([data[0], data[1]]));

        let res = RecordInfo::parse(data).map_err(|e| (e, Some(next_id)))?;

        Ok(res)
    }
}

#[cfg(feature = "alloc")]
#[derive(Debug, Clone)]
pub struct RecordInfo {
    pub next_entry: RecordId,
    pub record: Record,
}

#[cfg(feature = "alloc")]
impl RecordInfo {
    pub fn parse(data: &[u8]) -> Result<Self, RecordParseError> {
        let next_entry = RecordId::new_raw(u16::from_le_bytes([data[0], data[1]]));
        let data = &data[2..];
        Record::parse(data).map(|record| Self { next_entry, record })
    }
}
