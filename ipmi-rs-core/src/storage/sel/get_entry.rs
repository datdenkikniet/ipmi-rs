use core::num::NonZeroU16;

use nonmax::NonMaxU8;

use crate::connection::{EncodeIpmiCommand, IpmiCommand, NetFn};

use super::{Entry, ParseEntryError, RecordId};

#[derive(Clone, Debug, PartialEq)]
pub struct GetEntry {
    reservation_id: Option<NonZeroU16>,
    record_id: RecordId,
    offset: u8,
    bytes_to_read: Option<NonMaxU8>,
}

impl GetEntry {
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

#[derive(Clone, Debug, PartialEq)]
pub struct EntryInfo {
    pub next_entry: RecordId,
    pub entry: Entry,
}

impl IpmiCommand for GetEntry {
    type Output = EntryInfo;

    type Error = ParseEntryError;

    fn parse_success_response(data: &[u8]) -> Result<Self::Output, Self::Error> {
        if data.len() < 2 {
            return Err(ParseEntryError::NotEnoughData);
        }

        let next_entry = RecordId::new_raw(u16::from_le_bytes([data[0], data[1]]));
        let entry = Entry::parse(&data[2..])?;
        Ok(EntryInfo { next_entry, entry })
    }
}

impl EncodeIpmiCommand for GetEntry {
    const NETFN: NetFn = NetFn::Storage;
    const CMD: u8 = 0x43;

    fn request_data_len(&self) -> usize {
        6
    }

    fn write_request_data(&self, data: &mut [u8]) {
        data[0..2].copy_from_slice(
            &self
                .reservation_id
                .map(core::num::NonZeroU16::get)
                .unwrap_or(0)
                .to_le_bytes(),
        );
        data[2..4].copy_from_slice(&self.record_id.value().to_le_bytes());
        data[4] = self.offset;
        data[5] = self.bytes_to_read.map(|value| value.get()).unwrap_or(0xff);
    }
}
