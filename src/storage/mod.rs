mod sel;

pub use sel::{RecordId as SelRecordId, *};

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
