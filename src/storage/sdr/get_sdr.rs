use std::num::NonZeroU16;

use nonmax::NonMaxU8;

use crate::connection::{IpmiCommand, Message, NetFn, ParseResponseError};

use super::{record::Record, RecordId};

/// Get a device SDR.
///
/// This command must be used in accordance with the IPMI spec, i.e.
/// all SDRs must be obtained sequentially. It is recommended that you use
/// the [`Ipmi::sdrs`] function for this.
///
/// [`Ipmi::sdrs`]: crate::Ipmi::sdrs
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

impl Into<Message> for GetDeviceSdr {
    fn into(self) -> Message {
        let mut data = vec![0u8; 6];

        data[0..2].copy_from_slice(
            &self
                .reservation_id
                .map(NonZeroU16::get)
                .unwrap_or(0)
                .to_le_bytes(),
        );

        data[2..4].copy_from_slice(&self.record_id.value().to_le_bytes());
        data[4] = self.offset;
        data[5] = self.bytes_to_read.map(|v| v.get()).unwrap_or(0xFF);

        Message::new(NetFn::Storage, 0x23, data)
    }
}

impl IpmiCommand for GetDeviceSdr {
    type Output = RecordInfo;

    type Error = ();

    fn parse_response(
        completion_code: crate::connection::CompletionCode,
        data: &[u8],
    ) -> Result<Self::Output, ParseResponseError<Self::Error>> {
        Self::check_cc_success(completion_code)?;

        RecordInfo::parse(data).ok_or(ParseResponseError::NotEnoughData)
    }
}

#[derive(Debug, Clone)]
pub struct RecordInfo {
    pub next_entry: RecordId,
    pub record: Record,
}

impl RecordInfo {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 9 {
            return None;
        }

        let next_entry = RecordId::new_raw(u16::from_le_bytes([data[0], data[1]]));
        let data = &data[2..];

        Record::parse(data).map(|record| Self { next_entry, record })
    }
}
