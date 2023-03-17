use std::num::NonZeroU16;

use nonmax::NonMaxU8;

use crate::connection::{IpmiCommand, LogicalUnit, Message, NetFn, ParseResponseError};

use super::RecordId;

#[derive(Debug, Clone, Copy)]
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

impl Into<Message> for GetEntry {
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

impl IpmiCommand for GetEntry {
    type Output = EntryInfo;

    type Error = ();

    fn parse_response(
        completion_code: crate::connection::CompletionCode,
        data: &[u8],
    ) -> Result<Self::Output, ParseResponseError<Self::Error>> {
        Self::check_cc_success(completion_code)?;

        EntryInfo::parse(data).ok_or(ParseResponseError::NotEnoughData)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SensorOwner {
    I2C(u8),
    System(u8),
}

#[derive(Debug, Clone, Copy)]

pub enum EntityInstance {
    Physical(u8),
    LogicalContainer(u8),
}

bitflags::bitflags! {
    pub struct SensorInitialization: u8 {
        const SETTABLE = 1 << 7;
        const SCANNING = 1 << 6;
        const EVENTS = 1 << 5;
        const THRESHOLDS = 1 << 4;
        const HYSTERESIS = 1 << 3;
        const TYPE = 1 << 2;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct InitState {
    pub event_generation_enabled: bool,
    pub sensor_scanning_enabled: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum HysterisisCapability {
    NoneOrUnspecified,
    Readable,
    ReadableAndSettable,
    FixedAndUnreadable,
}

bitflags::bitflags! {
    struct ThresholdAssertEventMask: u16 {
        const UPPER_NON_RECOVERABLE_GOING_HIGH = 1 << 11;
        const UPPER_NON_RECOVERABLE_GOING_LOW = 1 << 10;
        const UPPER_CRITICAL_GOING_HIGH = 1 << 9;
        const UPPER_CRITICAL_GOING_LOW = 1 << 8;
        const UPPER_NON_CRITICAL_GOING_HIGH = 1 << 7;
        const UPPER_NON_CRITICAL_GOING_LOW = 1 << 6;
        const LOWER_NON_RECOVERABLE_GOING_HIGH = 1 << 5;
        const LOWER_NON_RECOVERABLE_GOING_LOW = 1 << 4;
        const LOWER_CRITICAL_GOING_HIGH = 1 << 3;
        const LOWER_CRITICAL_GOING_LOW = 1 << 2;
        const LOWER_NON_CRITICAL_GOING_HIGH = 1 << 1;
        const LOWER_NON_CRITICAL_GOING_LOW = 1 << 0;

    }
}

#[derive(Debug, Clone, Copy)]

pub struct ThresholdReadingMask {
    pub lower_non_recoverable_comparison: bool,
    pub lower_critical_comparison: bool,
    pub lower_non_critical_comparison: bool,
    pub upper_non_recoverable_comparison: bool,
    pub upper_critical_comparison: bool,
    pub uppser_non_critical_comparison: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum ThresholdAccessCapability {
    None,
    Readable(ThresholdReadingMask),
    ReadableAndSettable {
        readable: ThresholdReadingMask,
        settable: ThresholdReadingMask,
    },
    FixedAndUnreadable {
        supported: ThresholdReadingMask,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct SensorCapabilities {
    pub ignore: bool,
    pub auto_rearm: bool,
    pub hysterisis: HysterisisCapability,
    pub threshold_access: ThresholdAccessCapability,
}

#[derive(Debug, Clone)]
pub enum Record {
    FullSensor {
        owner: SensorOwner,
        owner_channel: u8,
        owner_lun: LogicalUnit,
        sensor_number: NonMaxU8,
        entity_id: u8,
        entity_instance: EntityInstance,
        initialization: SensorInitialization,
        init_state: InitState,
    },
    Unknown {
        data: Vec<u8>,
    },
}

impl Record {
    pub fn parse(data: &[u8]) -> Option<Self> {
        let _entry_id = RecordId::new_raw(u16::from_le_bytes([data[0], data[1]]));
        let _sdr_version_min = (data[2] & 0xF0) >> 4;
        let _sdr_version_maj = data[2] & 0x0F;
        let _record_type = data[3];
        let record_length = data[4];

        let record_data = &data[5..];

        if record_data.len() != record_length as usize {
            return None;
        } else {
            Some(Self::Unknown {
                data: data.to_vec(),
            })
        }
    }
}

#[derive(Debug, Clone)]
pub struct EntryInfo {
    pub next_entry: RecordId,
    pub record: Record,
}

impl EntryInfo {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 9 {
            return None;
        }

        let next_entry = RecordId::new_raw(u16::from_le_bytes([data[0], data[1]]));
        let data = &data[2..];

        Record::parse(data).map(|record| Self { next_entry, record })
    }
}
