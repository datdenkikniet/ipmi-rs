use std::num::NonZeroU16;

use nonmax::NonMaxU8;

use crate::{
    connection::{CompletionCode, IpmiCommand, Message, NetFn, ParseResponseError},
    LogOutput, Loggable,
};

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

impl Loggable for EntryInfo {
    fn log(&self, output: &LogOutput) {
        self.entry.log(output);
        crate::log!(output, "  Next entry: 0x{:04X}", self.next_entry.value());
    }
}

impl IpmiCommand for GetEntry {
    type Output = EntryInfo;

    type Error = ParseEntryError;

    fn parse_response(
        completion_code: CompletionCode,
        data: &[u8],
    ) -> Result<Self::Output, ParseResponseError<Self::Error>> {
        Self::check_cc_success(completion_code)?;

        if data.len() < 2 {
            return Err(ParseResponseError::NotEnoughData);
        }

        let next_entry = RecordId::new_raw(u16::from_le_bytes([data[0], data[1]]));
        let entry = Entry::parse(&data[2..])?;
        Ok(EntryInfo { next_entry, entry })
    }
}

impl Into<Message> for GetEntry {
    fn into(self) -> Message {
        let Self {
            reservation_id,
            record_id,
            offset,
            bytes_to_read,
        } = self;

        let mut data = vec![0u8; 6];

        data[0..2].copy_from_slice(&reservation_id.map(|v| v.get()).unwrap_or(0).to_be_bytes());
        data[2..4].copy_from_slice(&record_id.value().to_le_bytes());
        data[4] = offset;
        data[5] = bytes_to_read.map(|v| v.get()).unwrap_or(0xFF);

        Message::new(NetFn::Storage, 0x43, data)
    }
}
