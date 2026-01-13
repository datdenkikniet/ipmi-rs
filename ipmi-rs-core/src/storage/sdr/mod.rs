mod get_dev_sdr_info;
pub use get_dev_sdr_info::*;

mod get_sdr;
pub use get_sdr::{GetDeviceSdr, RecordInfo as SdrRecordInfo, *};

pub mod record;
pub use record::{ParseError as RecordParseError, Record};

mod get_info;
pub use get_info::{
    FreeSpace as SdrFreeSpace, GetRepositoryInfo as GetSdrRepositoryInfo,
    Operation as SdrOperation, RepositoryInfo as SdrRepositoryInfo,
};

mod get_alloc_info;
pub use get_alloc_info::{AllocInfo as SdrAllocInfo, GetAllocInfo as SdrGetAllocInfo};

pub mod event_reading_type_code;

mod sensor_type;
pub use sensor_type::SensorType;

mod event_offset;
pub use event_offset::decode_event;

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
