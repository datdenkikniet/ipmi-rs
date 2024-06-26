use crate::{
    connection::{Channel, LogicalUnit},
    fmt::LogItem,
    log_vec, Loggable,
};

use super::Timestamp;

mod get_alloc_info;
pub use get_alloc_info::{AllocInfo as SelAllocInfo, GetAllocInfo as GetSelAllocInfo};

mod get_entry;
pub use get_entry::{EntryInfo as SelEntryInfo, GetEntry as GetSelEntry};

mod get_info;
pub use get_info::{Command as SelCommand, GetInfo as GetSelInfo, Info as SelInfo};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RecordId(u16);

impl RecordId {
    pub const FIRST: Self = Self(0x0000);
    pub const LAST: Self = Self(0xFFFF);

    pub fn new(id: u16) -> Option<Self> {
        if RecordId(id) == Self::FIRST || RecordId(id) == Self::LAST {
            None
        } else {
            Some(Self(id))
        }
    }

    pub(crate) fn new_raw(id: u16) -> Self {
        RecordId(id)
    }

    pub fn value(&self) -> u16 {
        self.0
    }

    pub fn is_first(&self) -> bool {
        self == &Self::FIRST
    }

    pub fn is_last(&self) -> bool {
        self == &Self::LAST
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum SelRecordType {
    System,
    TimestampedOem(u8),
    NonTimestampedOem(u8),
    Unknown(u8),
}

impl From<u8> for SelRecordType {
    fn from(value: u8) -> Self {
        match value {
            0x02 => Self::System,
            0xC0..=0xDF => Self::TimestampedOem(value),
            0xE0..=0xFF => Self::NonTimestampedOem(value),
            v => Self::Unknown(v),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EventGenerator {
    RqSAAndLun {
        i2c_addr: u8,
        channel_number: Channel,
        lun: LogicalUnit,
    },
    SoftwareId {
        software_id: u8,
        channel_number: Channel,
    },
}

impl From<(u8, u8)> for EventGenerator {
    fn from(value: (u8, u8)) -> Self {
        let is_software_id = (value.0 & 0x1) == 0x1;
        let i2c_or_sid = (value.0 >> 1) & 0x7F;

        // NOTE(unwrap): value is in valid range due to mask.
        let channel_number = Channel::new((value.1 >> 4) & 0xF).unwrap();

        if is_software_id {
            Self::SoftwareId {
                software_id: i2c_or_sid,
                channel_number,
            }
        } else {
            let lun = LogicalUnit::from_low_bits(value.1);

            Self::RqSAAndLun {
                i2c_addr: i2c_or_sid,
                channel_number,
                lun,
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EventMessageRevision {
    V2_0,
    V1_0,
    Unknown(u8),
}

impl From<u8> for EventMessageRevision {
    fn from(value: u8) -> Self {
        match value {
            0x04 => Self::V2_0,
            0x03 => Self::V1_0,
            v => Self::Unknown(v),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EventDirection {
    Assert,
    Deassert,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Entry {
    System {
        record_id: RecordId,
        timestamp: Timestamp,
        generator_id: EventGenerator,
        event_message_format: EventMessageRevision,
        sensor_type: u8,
        sensor_number: u8,
        event_direction: EventDirection,
        event_type: u8,
        event_data: [u8; 3],
    },
    OemTimestamped {
        record_id: RecordId,
        ty: u8,
        timestamp: Timestamp,
        manufacturer_id: u32,
        data: [u8; 6],
    },
    OemNotTimestamped {
        record_id: RecordId,
        ty: u8,
        data: [u8; 13],
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParseEntryError {
    NotEnoughData,
    UnknownRecordType(u8),
}

impl Entry {
    pub fn parse(data: &[u8]) -> Result<Self, ParseEntryError> {
        if data.len() < 15 {
            return Err(ParseEntryError::NotEnoughData);
        }

        let record_id = RecordId(u16::from_le_bytes([data[0], data[1]]));
        let record_type = SelRecordType::from(data[2]);
        let timestamp = u32::from_le_bytes([data[3], data[4], data[5], data[6]]);

        match record_type {
            SelRecordType::System => {
                let generator_id = EventGenerator::from((data[7], data[8]));
                let event_message_format = EventMessageRevision::from(data[9]);
                let sensor_type = data[10];
                let sensor_number = data[11];
                let event_direction = if (data[12] & 0x80) == 0x80 {
                    EventDirection::Assert
                } else {
                    EventDirection::Deassert
                };
                let event_type = data[12] & 0x7F;
                let event_data = [data[13], data[14], data[15]];
                Ok(Self::System {
                    record_id,
                    timestamp: Timestamp::from(timestamp),
                    generator_id,
                    event_message_format,
                    sensor_type,
                    sensor_number,
                    event_direction,
                    event_type,
                    event_data,
                })
            }
            SelRecordType::TimestampedOem(v) => Ok(Self::OemTimestamped {
                record_id,
                ty: v,
                timestamp: Timestamp::from(timestamp),
                manufacturer_id: u32::from_le_bytes([data[7], data[8], data[9], 0]),
                data: [data[10], data[11], data[12], data[13], data[14], data[15]],
            }),
            SelRecordType::NonTimestampedOem(v) => Ok(Self::OemNotTimestamped {
                record_id,
                ty: v,
                data: [
                    data[3], data[4], data[5], data[6], data[7], data[8], data[9], data[10],
                    data[11], data[12], data[13], data[14], data[15],
                ],
            }),
            SelRecordType::Unknown(v) => Err(ParseEntryError::UnknownRecordType(v)),
        }
    }
}

impl Loggable for Entry {
    fn as_log(&self) -> Vec<LogItem> {
        match self {
            Entry::System {
                record_id,
                timestamp,
                generator_id,
                event_message_format,
                sensor_type,
                sensor_number,
                event_direction,
                event_type,
                event_data,
            } => {
                let format = match event_message_format {
                    EventMessageRevision::V2_0 => "2.0".into(),
                    EventMessageRevision::V1_0 => "1.0".into(),
                    EventMessageRevision::Unknown(v) => format!("Unknown (0x{:02X})", v),
                };

                let event_dir = match event_direction {
                    EventDirection::Assert => "Asserted",
                    EventDirection::Deassert => "Deasserted",
                };

                log_vec![
                    (0, "SEL entry"),
                    (1, "Record type", "System (0x02)"),
                    (1, "Record ID", format!("0x{:04X}", record_id.value())),
                    (1, "Time", timestamp),
                    (1, "Generator", format!("{:?}", generator_id)),
                    (1, "Format revision", format),
                    (1, "Sensor type", format!("0x{sensor_type:02X}")),
                    (1, "Sensor number", format!("0x{sensor_number:02X}")),
                    (1, "Assertion state", event_dir),
                    (1, "Event type", format!("0x{event_type:02X}")),
                    (1, "Data", format!("{event_data:02X?}")),
                ]
            }
            Entry::OemTimestamped {
                record_id,
                ty,
                timestamp,
                manufacturer_id,
                data,
            } => {
                log_vec![
                    (0, "SEL entry"),
                    (1, "Record type", format!("Timestamped OEM (0x{ty:08X})")),
                    (1, "Record ID", format!("0x{:04X}", record_id.value())),
                    (1, "Type", format!("{ty:02X}")),
                    (1, "Timestamp", timestamp),
                    (1, "Manufacturer ID", format!("{manufacturer_id:02X?}")),
                    (1, "Data", format!("{data:02X?}")),
                ]
            }
            Entry::OemNotTimestamped {
                record_id,
                ty,
                data,
            } => {
                log_vec![
                    (0, "SEL entry"),
                    (1, "Record type", format!("Not timestamp OEM (0x{ty:08X}")),
                    (1, "Record ID", format!("0x{:04X}", record_id.value())),
                    (1, "Type", format!("0x{ty:02X}")),
                    (1, "Data", format!("{data:02X?}"))
                ]
            }
        }
    }
}
