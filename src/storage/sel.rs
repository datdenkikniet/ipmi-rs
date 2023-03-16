use crate::{connection::LogicalUnit, LogOutput, Loggable};

use super::{Command, Timestamp};

#[derive(Debug, Clone)]
pub struct SelInfo {
    pub version_maj: u8,
    pub version_min: u8,
    pub entries: u16,
    pub bytes_free: u16,
    pub last_add_time: Timestamp,
    pub last_del_time: Timestamp,
    pub overflow: bool,
    pub supported_cmds: Vec<Command>,
}

impl SelInfo {
    pub fn from_data(data: &[u8]) -> Option<Self> {
        if data.len() != 14 {
            return None;
        }

        let version_maj = data[0] & 0xF;
        let version_min = (data[0] >> 4) & 0xF;

        let entries = u16::from_le_bytes([data[1], data[2]]);
        let free = u16::from_le_bytes([data[3], data[4]]);

        let last_add_time = u32::from_le_bytes([data[5], data[6], data[7], data[8]]);
        let last_del_time = u32::from_le_bytes([data[9], data[10], data[11], data[12]]);
        let overflow = data[13] & 0x80 == 0x80;

        let mut supported_cmds = Vec::with_capacity(4);

        if data[13] & 0x08 == 0x08 {
            supported_cmds.push(Command::ClearSel);
        }
        if data[13] & 0x04 == 0x04 {
            supported_cmds.push(Command::PartialAddSelEntry);
        }
        if data[13] & 0x02 == 0x02 {
            supported_cmds.push(Command::ReserveSel);
        }
        if data[13] & 0x01 == 0x01 {
            supported_cmds.push(Command::GetSelAllocInfo);
        }

        Some(SelInfo {
            version_maj,
            version_min,
            entries,
            bytes_free: free,
            last_add_time: Timestamp(last_add_time),
            last_del_time: Timestamp(last_del_time),
            overflow,
            supported_cmds,
        })
    }
}

impl Loggable for SelInfo {
    fn log(&self, level: LogOutput) {
        use crate::log;
        let (ver_maj, ver_min) = (self.version_maj, self.version_min);

        log!(level, "SEL information:");
        log!(level, "  Version:        {}.{}", ver_maj, ver_min);
        log!(level, "  Entries:        {}", self.entries);
        log!(level, "  Bytes free:     {}", self.bytes_free);
        log!(level, "  Last addition:  {}", self.last_add_time);
        log!(level, "  Last erase:     {}", self.last_del_time);

        let supported_cmds: Vec<_> = self
            .supported_cmds
            .iter()
            .map(|cmd| match cmd {
                Command::GetSelAllocInfo => "Get Alloc Info",
                Command::ClearSel => "Clear",
                Command::PartialAddSelEntry => "Partial Add",
                Command::ReserveSel => "Reserve",
                _ => unreachable!(),
            })
            .collect();

        log!(level, "  Supported cmds: {:?}", supported_cmds);
    }
}

#[derive(Debug, Clone)]
pub struct SelAllocInfo {
    pub num_alloc_units: u16,
    pub alloc_unit_size: u16,
    pub num_free_units: u16,
    pub largest_free_blk: u16,
    pub max_record_size: u8,
}

impl SelAllocInfo {
    pub fn from_data(data: &[u8]) -> Option<Self> {
        if data.len() < 8 {
            return None;
        }

        let num_alloc_units = u16::from_le_bytes([data[0], data[1]]);
        let alloc_unit_size = u16::from_le_bytes([data[2], data[3]]);
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

impl Loggable for SelAllocInfo {
    fn log(&self, level: LogOutput) {
        use crate::log;
        log!(level, "SEL Allocation info:");
        log!(level, "  # of units:         {}", self.num_alloc_units);
        log!(level, "  Unit size:          {}", self.alloc_unit_size);
        log!(level, "  # free units:       {}", self.num_free_units);
        log!(level, "  Largest free block: {}", self.largest_free_blk);
        log!(level, "  Max record size:    {}", self.max_record_size)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelRecordId(u16);

impl SelRecordId {
    pub const FIRST: Self = Self(0x0000);
    pub const LAST: Self = Self(0xFFFF);

    pub fn new(id: u16) -> Option<Self> {
        if id == Self::FIRST.0 {
            None
        } else if id == Self::LAST.0 {
            None
        } else {
            Some(Self(id))
        }
    }

    pub(crate) fn new_raw(id: u16) -> Self {
        SelRecordId(id)
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
pub enum SelEventGenerator {
    RqSAAndLun {
        i2c_addr: u8,
        channel_number: u8,
        lun: LogicalUnit,
    },
    SoftwareId {
        software_id: u8,
        channel_number: u8,
    },
}

impl From<(u8, u8)> for SelEventGenerator {
    fn from(value: (u8, u8)) -> Self {
        let is_software_id = (value.0 & 0x1) == 0x1;
        let i2c_or_sid = (value.0 >> 1) & 0x7F;
        let channel_number = (value.1 >> 4) & 0xF;

        if is_software_id {
            Self::SoftwareId {
                software_id: i2c_or_sid,
                channel_number,
            }
        } else {
            let lun = match value.1 & 0x3 {
                0 => LogicalUnit::ZERO,
                1 => LogicalUnit::ONE,
                2 => LogicalUnit::TWO,
                3 => LogicalUnit::THREE,
                _ => unreachable!(),
            };

            Self::RqSAAndLun {
                i2c_addr: i2c_or_sid,
                channel_number,
                lun,
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelEventMessageRevision {
    V2_0,
    V1_0,
    Unknown(u8),
}

impl From<u8> for SelEventMessageRevision {
    fn from(value: u8) -> Self {
        match value {
            0x04 => Self::V2_0,
            0x03 => Self::V1_0,
            v => Self::Unknown(v),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelEventDirection {
    Assert,
    Deassert,
}

#[derive(Debug, Clone)]
pub enum SelEntry {
    System {
        record_id: SelRecordId,
        timestamp: Timestamp,
        generator_id: SelEventGenerator,
        event_message_format: SelEventMessageRevision,
        sensor_type: u8,
        sensor_number: u8,
        event_direction: SelEventDirection,
        event_type: u8,
        event_data: [u8; 3],
    },
    OemTimestamped {
        record_id: SelRecordId,
        ty: u8,
        timestamp: Timestamp,
        manufacturer_id: u32,
        data: [u8; 6],
    },
    OemNotTimestamped {
        record_id: SelRecordId,
        ty: u8,
        data: [u8; 13],
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParseSelEntryError {
    NotEnoughData,
    UnknownRecordType(u8),
}

impl SelEntry {
    pub fn from_data(data: &[u8]) -> Result<Self, ParseSelEntryError> {
        if data.len() < 15 {
            return Err(ParseSelEntryError::NotEnoughData);
        }

        let record_id = SelRecordId(u16::from_le_bytes([data[0], data[1]]));
        let record_type = SelRecordType::from(data[2]);
        let timestamp = u32::from_le_bytes([data[3], data[4], data[5], data[6]]);

        match record_type {
            SelRecordType::System => {
                let generator_id = SelEventGenerator::from((data[7], data[8]));
                let event_message_format = SelEventMessageRevision::from(data[9]);
                let sensor_type = data[10];
                let sensor_number = data[11];
                let event_direction = if (data[12] & 0x80) == 0x80 {
                    SelEventDirection::Assert
                } else {
                    SelEventDirection::Deassert
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
                manufacturer_id: u32::from_be_bytes([data[7], data[8], data[9], 0]),
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
            SelRecordType::Unknown(v) => Err(ParseSelEntryError::UnknownRecordType(v)),
        }
    }
}

impl Loggable for SelEntry {
    fn log(&self, output: LogOutput) {
        use crate::log;
        log!(output, "Sel Entry:");
        match self {
            SelEntry::System {
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
                    SelEventMessageRevision::V2_0 => "2.0".into(),
                    SelEventMessageRevision::V1_0 => "1.0".into(),
                    SelEventMessageRevision::Unknown(v) => format!("Unknown (0x{:02X})", v),
                };

                let event_dir = match event_direction {
                    SelEventDirection::Assert => "Asserted",
                    SelEventDirection::Deassert => "Deasserted",
                };

                log!(output, "  Record type:     System (0x02)");
                log!(output, "  Record ID:       0x{:04X}", record_id.value());
                log!(output, "  Time:            {}", timestamp);
                log!(output, "  Generator:       {:?}", generator_id);
                log!(output, "  Format revision: {format}");
                log!(output, "  Sensor type:     0x{:02X}", sensor_type);
                log!(output, "  Sensor number:   0x{:02X}", sensor_number);
                log!(output, "  Assertion state: {event_dir}");
                log!(output, "  Event type:      0x{:02X}", event_type);
                log!(output, "  Data:            {:02X?}", event_data);
            }
            SelEntry::OemTimestamped {
                record_id,
                ty,
                timestamp,
                manufacturer_id,
                data,
            } => {
                log!(output, "  Record type:     Timestamped OEM (0x{:08X})", ty);
                log!(output, "  Record ID:       0x{:04X}", record_id.value());
                log!(output, "  Type:            {:02X}", ty);
                log!(output, "  Timstamp:        {timestamp}");
                log!(output, "  Manufacturer ID: {:02X?}", manufacturer_id);
                log!(output, "  Data:            {:02X?}", data);
            }
            SelEntry::OemNotTimestamped {
                record_id,
                ty,
                data,
            } => {
                #[rustfmt::skip]
                log!(output, "  Record type:     Not Timestamped OEM (0x{:08X})", ty);
                log!(output, "  Record ID: 0x{:04X}", record_id.value());
                log!(output, "  Type:      0x{:02X}", ty);
                log!(output, "  Data:      {:02X?}", data);
            }
        }
    }
}
