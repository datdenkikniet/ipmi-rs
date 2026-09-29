mod get_dev_sdr_info;
pub use get_dev_sdr_info::*;

mod get_sdr;
pub use get_sdr::GetDeviceSdr;
#[cfg(feature = "alloc")]
pub use get_sdr::RecordInfo as SdrRecordInfo;

#[cfg(feature = "alloc")]
pub mod record;
#[cfg(feature = "alloc")]
pub use record::{ParseError as RecordParseError, Record};

mod get_info;
#[cfg(feature = "alloc")]
pub use get_info::RepositoryInfo as SdrRepositoryInfo;
pub use get_info::{
    FreeSpace as SdrFreeSpace, GetRepositoryInfo as GetSdrRepositoryInfo, Operation as SdrOperation,
};

mod get_alloc_info;
pub use get_alloc_info::{AllocInfo as SdrAllocInfo, GetAllocInfo as SdrGetAllocInfo};

pub mod event_reading_type_code;

mod sensor_type;
pub use sensor_type::SensorType;

mod event_offset;
pub use event_offset::decode_event;

/// Sensor number, excluding the reserved `0xff` value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SensorNumber(nonmax::NonMaxU8);

impl SensorNumber {
    /// Create a sensor number.
    pub const fn new(value: nonmax::NonMaxU8) -> Self {
        Self(value)
    }

    /// Get the wire value.
    pub const fn get(self) -> u8 {
        self.0.get()
    }
}

mod event_data;
pub use event_data::{EventData, EventData2Type, EventData3Type};

mod units;
pub use units::Unit;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct RecordId(u16);

impl RecordId {
    pub const FIRST: Self = Self(0);
    pub const LAST: Self = Self(0xFFFF);

    pub fn new_raw(value: u16) -> Self {
        Self(value)
    }

    pub fn is_first(&self) -> bool {
        self.0 == Self::FIRST.0
    }

    pub fn is_last(&self) -> bool {
        self.0 == Self::LAST.0
    }

    pub fn value(&self) -> u16 {
        self.0
    }
}
