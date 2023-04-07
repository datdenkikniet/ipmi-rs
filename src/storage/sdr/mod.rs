mod get_sdr;
pub use get_sdr::{GetDeviceSdr, RecordInfo as SdrRecordInfo, *};

pub mod record;

mod get_info;
pub use get_info::{
    FreeSpace as SdrFreeSpace, GetRepositoryInfo as GetSdrRepositoryInfo,
    Operation as SdrOperation, RepositoryInfo as SdrRepositoryInfo,
};

mod get_alloc_info;
pub use get_alloc_info::{AllocInfo, GetAllocInfo as GetSdrAllocInfo};

pub mod event_reading_type_code;

mod sensor_type;
pub use sensor_type::SensorType;

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
