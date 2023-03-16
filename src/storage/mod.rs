mod sel;
use std::num::NonZeroU16;

use nonmax::NonMaxU8;
pub use sel::{
    ParseSelEntryError, SelAllocInfo, SelEntry, SelEventDirection, SelEventGenerator,
    SelEventMessageRevision, SelInfo, SelRecordId,
};

use crate::connection::NetFns;

#[derive(Clone, Debug, PartialEq)]
pub struct GetSelEntry {
    pub reservation_id: Option<NonZeroU16>,
    pub record_id: SelRecordId,
    pub offset: u8,
    pub bytes_to_read: Option<NonMaxU8>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    GetSelInfo,
    GetSelAllocInfo,
    ReserveSel,
    GetSelEntry(Option<GetSelEntry>),
    PartialAddSelEntry,
    ClearSel,
    Unknown(u8, Vec<u8>),
}

impl Command {
    pub fn request_data(&self) -> Vec<u8> {
        match self {
            Command::GetSelEntry(Some(GetSelEntry {
                reservation_id,
                record_id,
                offset,
                bytes_to_read,
            })) => {
                let mut data = vec![0u8; 6];

                data[0..2]
                    .copy_from_slice(&reservation_id.map(|v| v.get()).unwrap_or(0).to_be_bytes());
                data[2..4].copy_from_slice(&record_id.value().to_le_bytes());
                data[4] = *offset;
                data[5] = bytes_to_read.map(|v| v.get()).unwrap_or(0xFF);

                data
            }
            _ => Vec::new(),
        }
    }

    pub fn cmd_id(&self) -> u8 {
        match self {
            Command::GetSelInfo => 0x40,
            Command::GetSelAllocInfo => 0x41,
            Command::ReserveSel => 0x42,
            Command::GetSelEntry { .. } => 0x43,
            Command::PartialAddSelEntry => 0x45,
            Command::ClearSel => 0x47,
            Command::Unknown(v, _) => *v,
        }
    }

    pub fn request_parts(&self) -> (u8, Vec<u8>) {
        (self.cmd_id(), self.request_data())
    }

    pub fn from_response_parts(cmd: u8, data: &[u8]) -> Self {
        match cmd {
            0x40 => Self::GetSelInfo,
            0x41 => Self::GetSelAllocInfo,
            0x42 => Self::ReserveSel,
            0x43 => Self::GetSelEntry(None),
            0x45 => Self::PartialAddSelEntry,
            0x47 => Self::ClearSel,
            v => Self::Unknown(v, data.iter().map(Clone::clone).collect()),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NetFn {
    is_response: bool,
    cmd: Command,
}

impl NetFn {
    const fn new(is_response: bool, cmd: Command) -> Self {
        Self { is_response, cmd }
    }
}

impl NetFns for NetFn {
    type Command = Command;

    fn request(cmd: Command) -> Self {
        Self::new(false, cmd)
    }

    fn response(cmd: Command) -> Self {
        Self::new(true, cmd)
    }

    fn is_response(&self) -> bool {
        self.is_response
    }

    fn cmd(&self) -> Command {
        self.cmd.clone()
    }

    fn request_data(&self) -> Vec<u8> {
        self.cmd().request_data()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Timestamp(u32);

impl core::fmt::Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[cfg(feature = "time")]
        {
            let timestamp = time::OffsetDateTime::from_unix_timestamp(self.0 as i64).unwrap();

            let time = timestamp
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap();

            write!(f, "{}", time)
        }

        #[cfg(not(feature = "time"))]
        write!(f, "{}", self.0)
    }
}

impl From<u32> for Timestamp {
    fn from(value: u32) -> Self {
        Self(value)
    }
}
