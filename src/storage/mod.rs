mod sel;

use std::num::NonZeroU16;

pub use sel::{RecordId as SelRecordId, *};

pub mod sdr;
pub use sdr::{RecordId as SdrRecordId, *};

use crate::{LogOutput, Loggable};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Timestamp(u32);

impl core::fmt::Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 == 0 {
            write!(f, "Unknown")
        } else {
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
}

impl From<u32> for Timestamp {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone)]
pub struct AllocInfo {
    pub num_alloc_units: Option<NonZeroU16>,
    pub alloc_unit_size: Option<NonZeroU16>,
    pub num_free_units: u16,
    pub largest_free_blk: u16,
    pub max_record_size: u8,
}

impl AllocInfo {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 8 {
            return None;
        }

        let num_alloc_units = NonZeroU16::new(u16::from_le_bytes([data[0], data[1]]));
        let alloc_unit_size = NonZeroU16::new(u16::from_le_bytes([data[2], data[3]]));
        let num_free_units = u16::from_le_bytes([data[4], data[5]]);
        let largest_free_blk = u16::from_le_bytes([data[6], data[7]]);
        let max_record_size = data[8];

        Some(Self {
            num_alloc_units,
            alloc_unit_size,
            num_free_units,
            largest_free_blk,
            max_record_size,
        })
    }
}

impl Loggable for AllocInfo {
    fn log(&self, level: LogOutput) {
        use crate::log;

        let unspecified_if_zero = |v: Option<NonZeroU16>| {
            if let Some(v) = v {
                format!("{}", v.get())
            } else {
                "Unspecified".into()
            }
        };

        let num_alloc_units = unspecified_if_zero(self.num_alloc_units);
        let alloc_unit_size = unspecified_if_zero(self.alloc_unit_size);

        log!(level, "  # of units:         {num_alloc_units}");
        log!(level, "  Unit size:          {alloc_unit_size}");
        log!(level, "  # free units:       {}", self.num_free_units);
        log!(level, "  Largest free block: {}", self.largest_free_blk);
        log!(level, "  Max record size:    {}", self.max_record_size)
    }
}
