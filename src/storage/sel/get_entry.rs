use std::num::NonZeroU16;

use nonmax::NonMaxU8;

use crate::{
    connection::{CompletionCode, IpmiCommand, Message, NetFn, ParseResponseError},
    Loggable,
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
    fn into_log(&self) -> Vec<crate::fmt::LogItem> {
        let mut log_output = self.entry.into_log();

        let value = format!("0x{:04X}", self.next_entry.value());
        log_output.push((1, "Next entry", value).into());
        log_output
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
        let entry = Entry::parse(&data[2..]).map_err(|e| ParseResponseError::Parse(e))?;
        Ok(EntryInfo { next_entry, entry })
    }
}

impl From<GetEntry> for Message {
    fn from(value: GetEntry) -> Self {
        let GetEntry {
            reservation_id,
            record_id,
            offset,
            bytes_to_read,
        } = value;

        let mut data = vec![0u8; 6];

        data[0..2].copy_from_slice(&reservation_id.map(|v| v.get()).unwrap_or(0).to_be_bytes());
        data[2..4].copy_from_slice(&record_id.value().to_le_bytes());
        data[4] = offset;
        data[5] = bytes_to_read.map(|v| v.get()).unwrap_or(0xFF);

        Message::new_request(NetFn::Storage, 0x43, data)
    }
}
