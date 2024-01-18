mod full_sensor_record;
pub use full_sensor_record::FullSensorRecord;

mod compact_sensor_record;
mod fru_device_locator;

pub use compact_sensor_record::CompactSensorRecord;

use nonmax::NonMaxU8;

use crate::storage::sdr::record::fru_device_locator::FruDeviceLocator;
use crate::{connection::LogicalUnit, Loggable};

use super::{event_reading_type_code::EventReadingTypeCodes, RecordId, SensorType, Unit};

pub trait SensorRecord {
    fn common(&self) -> &SensorRecordCommon;

    fn capabilities(&self) -> &SensorCapabilities {
        &self.common().capabilities
    }

    fn id_string(&self) -> &SensorId {
        &self.common().sensor_id
    }

    fn direction(&self) -> Direction;

    fn sensor_number(&self) -> SensorNumber {
        self.common().key.sensor_number
    }

    fn entity_id(&self) -> u8 {
        self.common().entity_id
    }

    fn key_data(&self) -> &SensorKey {
        &self.common().key
    }
}

#[derive(Debug)]
pub struct Value {
    units: SensorUnits,
    value: f32,
}

impl Value {
    pub fn new(units: SensorUnits, value: f32) -> Self {
        Self { units, value }
    }

    pub fn display(&self, short: bool) -> String {
        if self.units.is_percentage {
            format!("{:.2} %", self.value)
        } else {
            // TODO: use Modifier unit and rate units
            // somehow here
            self.units.base_unit.display(short, self.value)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SensorKey {
    pub owner_id: SensorOwner,
    pub owner_channel: u8,
    pub fru_inv_device_owner_lun: LogicalUnit,
    pub owner_lun: LogicalUnit,
    pub sensor_number: SensorNumber,
}

impl SensorKey {
    pub fn parse(record_data: &[u8]) -> Option<Self> {
        if record_data.len() != 3 {
            return None;
        }

        let owner_id = SensorOwner::from(record_data[0]);
        let owner_channel_fru_lun = record_data[1];
        let owner_channel = (owner_channel_fru_lun & 0xF0) >> 4;
        let fru_inv_device_owner_lun =
            LogicalUnit::try_from((owner_channel_fru_lun >> 2) & 0x3).unwrap();
        let owner_lun = LogicalUnit::try_from(owner_channel_fru_lun & 0x3).unwrap();

        let sensor_number = SensorNumber(NonMaxU8::new(record_data[2])?);

        Some(Self {
            owner_id,
            owner_channel,
            fru_inv_device_owner_lun,
            owner_lun,
            sensor_number,
        })
    }
}

impl SensorKey {
    fn log_into(&self, level: usize, log: &mut Vec<crate::fmt::LogItem>) {
        let sensor_owner = match self.owner_id {
            SensorOwner::I2C(addr) => format!("I2C @ 0x{:02X}", addr),
            SensorOwner::System(addr) => format!("System @ 0x{:02X}", addr),
        };

        log.push((level, "Sensor owner", sensor_owner).into());
        log.push((level, "Owner channel", self.owner_channel).into());
        log.push((level, "Owner LUN", self.owner_lun.value()).into());
        log.push((level, "Sensor number", self.sensor_number.get()).into());
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SensorOwner {
    I2C(u8),
    System(u8),
}

impl From<u8> for SensorOwner {
    fn from(value: u8) -> Self {
        let id = (value & 0xFE) >> 1;

        if (value & 1) == 1 {
            Self::System(id)
        } else {
            Self::I2C(id)
        }
    }
}

impl Into<u8> for SensorOwner {
    fn into(self) -> u8 {
        match self {
            Self::I2C(id) => (id << 1) & 0xFE,
            Self::System(id) => ((id << 1) & 0xFE) | 1,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EntityRelativeTo {
    System,
    Device,
}

#[derive(Debug, Clone, Copy)]

pub enum EntityInstance {
    Physical {
        relative: EntityRelativeTo,
        instance_number: u8,
    },
    LogicalContainer {
        relative: EntityRelativeTo,
        instance_number: u8,
    },
}

impl From<u8> for EntityInstance {
    fn from(value: u8) -> Self {
        let instance_number = value & 0x7F;
        let relative = match instance_number {
            0x00..=0x5F => EntityRelativeTo::System,
            0x60..=0x7F => EntityRelativeTo::Device,
            _ => unreachable!(),
        };

        if (value & 0x80) == 0x80 {
            Self::LogicalContainer {
                relative,
                instance_number,
            }
        } else {
            Self::Physical {
                relative,
                instance_number,
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SensorInitialization {
    pub settable: bool,
    pub scanning: bool,
    pub events: bool,
    pub thresholds: bool,
    pub hysteresis: bool,
    pub sensor_type: bool,
    pub event_generation_enabled_on_startup: bool,
    pub sensor_scanning_enabled_on_startup: bool,
}

impl From<u8> for SensorInitialization {
    fn from(value: u8) -> Self {
        bitflags::bitflags! {
            pub struct Flags: u8 {
                const SETTABLE = 1 << 7;
                const SCANNING = 1 << 6;
                const EVENTS = 1 << 5;
                const THRESHOLDS = 1 << 4;
                const HYSTERESIS = 1 << 3;
                const TYPE = 1 << 2;
                const EVENTGEN_ON_STARTUP = 1 << 1;
                const SCANNING_ON_STARTUP = 1 << 0;
            }
        }

        let flags = Flags::from_bits_truncate(value);

        Self {
            settable: flags.contains(Flags::SETTABLE),
            scanning: flags.contains(Flags::SCANNING),
            events: flags.contains(Flags::EVENTS),
            thresholds: flags.contains(Flags::THRESHOLDS),
            hysteresis: flags.contains(Flags::THRESHOLDS),
            sensor_type: flags.contains(Flags::TYPE),
            event_generation_enabled_on_startup: flags.contains(Flags::EVENTGEN_ON_STARTUP),
            sensor_scanning_enabled_on_startup: flags.contains(Flags::SCANNING_ON_STARTUP),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum HysteresisCapability {
    NoneOrUnspecified,
    Readable,
    ReadableAndSettable,
    FixedAndUnreadable,
}

bitflags::bitflags! {
    pub struct ThresholdAssertEventMask: u16 {
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

impl ThresholdAssertEventMask {
    pub fn for_kind(&self, kind: ThresholdKind) -> &[EventKind] {
        static BOTH: [EventKind; 2] = [EventKind::GoingHigh, EventKind::GoingLow];
        static HIGH: [EventKind; 1] = [EventKind::GoingHigh];
        static LOW: [EventKind; 1] = [EventKind::GoingLow];
        static NONE: [EventKind; 0] = [];

        let (low, high) = match kind {
            ThresholdKind::LowerNonCritical => (
                self.contains(Self::LOWER_NON_CRITICAL_GOING_LOW),
                self.contains(Self::LOWER_NON_CRITICAL_GOING_HIGH),
            ),
            ThresholdKind::LowerCritical => (
                self.contains(Self::LOWER_CRITICAL_GOING_LOW),
                self.contains(Self::LOWER_CRITICAL_GOING_HIGH),
            ),
            ThresholdKind::LowerNonRecoverable => (
                self.contains(Self::LOWER_NON_RECOVERABLE_GOING_LOW),
                self.contains(Self::LOWER_NON_RECOVERABLE_GOING_HIGH),
            ),
            ThresholdKind::UpperNonCritical => (
                self.contains(Self::UPPER_NON_CRITICAL_GOING_LOW),
                self.contains(Self::UPPER_NON_CRITICAL_GOING_HIGH),
            ),
            ThresholdKind::UpperCritical => (
                self.contains(Self::UPPER_CRITICAL_GOING_LOW),
                self.contains(Self::UPPER_CRITICAL_GOING_HIGH),
            ),
            ThresholdKind::UpperNonRecoverable => (
                self.contains(Self::UPPER_NON_RECOVERABLE_GOING_LOW),
                self.contains(Self::UPPER_NON_RECOVERABLE_GOING_HIGH),
            ),
        };

        if low && high {
            &BOTH
        } else if low {
            &LOW
        } else if high {
            &HIGH
        } else {
            &NONE
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EventKind {
    GoingHigh,
    GoingLow,
}

#[derive(Debug, Clone, Copy)]

pub struct Thresholds {
    pub lower_non_recoverable: bool,
    pub lower_critical: bool,
    pub lower_non_critical: bool,
    pub upper_non_recoverable: bool,
    pub upper_critical: bool,
    pub upper_non_critical: bool,
}

impl Thresholds {
    pub fn for_kind(&self, kind: ThresholdKind) -> bool {
        match kind {
            ThresholdKind::LowerNonCritical => self.lower_non_critical,
            ThresholdKind::LowerCritical => self.lower_critical,
            ThresholdKind::LowerNonRecoverable => self.lower_non_recoverable,
            ThresholdKind::UpperNonCritical => self.upper_non_critical,
            ThresholdKind::UpperCritical => self.upper_critical,
            ThresholdKind::UpperNonRecoverable => self.upper_non_recoverable,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThresholdKind {
    LowerNonCritical,
    LowerCritical,
    LowerNonRecoverable,
    UpperNonCritical,
    UpperCritical,
    UpperNonRecoverable,
}

impl ThresholdKind {
    pub fn variants() -> impl Iterator<Item = Self> {
        [
            Self::LowerNonCritical,
            Self::LowerCritical,
            Self::LowerNonRecoverable,
            Self::UpperNonCritical,
            Self::UpperCritical,
            Self::UpperNonRecoverable,
        ]
        .into_iter()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Threshold {
    pub kind: ThresholdKind,
    pub readable: bool,
    pub settable: bool,
    pub event_assert_going_high: bool,
    pub event_assert_going_low: bool,
    pub event_deassert_going_high: bool,
    pub event_deassert_going_low: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum ThresholdAccessCapability {
    None,
    Readable {
        readable: Thresholds,
        values: Thresholds,
    },
    ReadableAndSettable {
        readable: Thresholds,
        values: Thresholds,
        settable: Thresholds,
    },
    FixedAndUnreadable {
        supported: Thresholds,
    },
}

impl ThresholdAccessCapability {
    pub fn readable(&self, kind: ThresholdKind) -> bool {
        match self {
            ThresholdAccessCapability::Readable { readable, .. } => readable.for_kind(kind),
            ThresholdAccessCapability::ReadableAndSettable { readable, .. } => {
                readable.for_kind(kind)
            }
            _ => false,
        }
    }

    pub fn settable(&self, kind: ThresholdKind) -> bool {
        match self {
            ThresholdAccessCapability::ReadableAndSettable { settable, .. } => {
                settable.for_kind(kind)
            }
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SensorCapabilities {
    pub ignore: bool,
    pub auto_rearm: bool,
    // TODO: make a type
    pub event_message_control: u8,
    pub hysteresis: HysteresisCapability,
    pub threshold_access: ThresholdAccessCapability,
    pub assertion_threshold_events: ThresholdAssertEventMask,
    pub deassertion_threshold_events: ThresholdAssertEventMask,
}

impl SensorCapabilities {
    pub fn new(
        caps: u8,
        assert_lower_thrsd: u16,
        deassert_upper_thrshd: u16,
        discrete_rd_thrsd_set_thrshd_read: u16,
    ) -> Self {
        let ignore = (caps & 0x80) == 0x80;
        let auto_rearm = (caps & 0x40) == 0x40;
        let hysteresis = match caps & 0x30 >> 4 {
            0b00 => HysteresisCapability::NoneOrUnspecified,
            0b01 => HysteresisCapability::Readable,
            0b10 => HysteresisCapability::ReadableAndSettable,
            0b11 => HysteresisCapability::FixedAndUnreadable,
            _ => unreachable!(),
        };
        let event_message_control = caps & 0b11;

        let assertion_event_mask = ThresholdAssertEventMask::from_bits_truncate(assert_lower_thrsd);
        let deassertion_event_mask =
            ThresholdAssertEventMask::from_bits_truncate(deassert_upper_thrshd);

        let threshold_read_value_mask = Thresholds {
            lower_non_recoverable: ((assert_lower_thrsd >> 14) & 0x1) == 1,
            lower_critical: ((assert_lower_thrsd >> 13) & 0x1) == 1,
            lower_non_critical: ((assert_lower_thrsd >> 12) & 0x1) == 1,
            upper_non_recoverable: ((deassert_upper_thrshd >> 14) & 0x1) == 1,
            upper_critical: ((deassert_upper_thrshd >> 14) & 0x1) == 1,
            upper_non_critical: ((deassert_upper_thrshd >> 14) & 0x1) == 1,
        };

        let threshold_set_mask = Thresholds {
            upper_non_recoverable: ((discrete_rd_thrsd_set_thrshd_read >> 13) & 0x1) == 1,
            upper_critical: ((discrete_rd_thrsd_set_thrshd_read >> 12) & 0x1) == 1,
            upper_non_critical: ((discrete_rd_thrsd_set_thrshd_read >> 11) & 0x1) == 1,
            lower_non_recoverable: ((discrete_rd_thrsd_set_thrshd_read >> 10) & 0x1) == 1,
            lower_critical: ((discrete_rd_thrsd_set_thrshd_read >> 9) & 0x1) == 1,
            lower_non_critical: ((discrete_rd_thrsd_set_thrshd_read >> 8) & 0x1) == 1,
        };

        let threshold_read_mask = Thresholds {
            upper_non_recoverable: ((discrete_rd_thrsd_set_thrshd_read >> 5) & 0x1) == 1,
            upper_critical: ((discrete_rd_thrsd_set_thrshd_read >> 4) & 0x1) == 1,
            upper_non_critical: ((discrete_rd_thrsd_set_thrshd_read >> 3) & 0x1) == 1,
            lower_non_recoverable: ((discrete_rd_thrsd_set_thrshd_read >> 2) & 0x1) == 1,
            lower_critical: ((discrete_rd_thrsd_set_thrshd_read >> 1) & 0x1) == 1,
            lower_non_critical: (discrete_rd_thrsd_set_thrshd_read & 0x1) == 1,
        };

        let threshold_access_support = match (caps & 0xC) >> 2 {
            0b00 => ThresholdAccessCapability::None,
            0b01 => ThresholdAccessCapability::Readable {
                readable: threshold_read_mask,
                values: threshold_read_value_mask,
            },
            0b10 => ThresholdAccessCapability::ReadableAndSettable {
                readable: threshold_read_mask,
                values: threshold_read_value_mask,
                settable: threshold_set_mask,
            },
            0b11 => ThresholdAccessCapability::FixedAndUnreadable {
                supported: threshold_read_mask,
            },
            _ => unreachable!(),
        };

        Self {
            ignore,
            auto_rearm,
            hysteresis,
            event_message_control,
            threshold_access: threshold_access_support,
            assertion_threshold_events: assertion_event_mask,
            deassertion_threshold_events: deassertion_event_mask,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DataFormat {
    Unsigned,
    OnesComplement,
    TwosComplement,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RateUnit {
    Microsecond,
    Millisecond,
    Second,
    Minute,
    Hour,
    Day,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModifierUnit {
    BasUnitDivByModifier(Unit),
    BaseUnitMulByModifier(Unit),
}

#[derive(Debug, Clone, Copy)]
pub struct SensorUnits {
    pub rate: Option<RateUnit>,
    pub modifier: Option<ModifierUnit>,
    pub is_percentage: bool,
    pub base_unit: Unit,
}

impl SensorUnits {
    pub fn from(sensor_units_1: u8, base_unit: u8, modifier_unit: u8) -> Self {
        let rate = match (sensor_units_1 >> 3) & 0b111 {
            0b000 => None,
            0b001 => Some(RateUnit::Microsecond),
            0b010 => Some(RateUnit::Millisecond),
            0b011 => Some(RateUnit::Second),
            0b100 => Some(RateUnit::Minute),
            0b101 => Some(RateUnit::Hour),
            0b110 => Some(RateUnit::Day),
            0b111 => None,
            _ => unreachable!(),
        };

        let base_unit = Unit::from(base_unit);

        let modifier_unit = Unit::from(modifier_unit);

        let modifier = match (sensor_units_1 >> 1) & 0b11 {
            0b00 => None,
            0b01 => Some(ModifierUnit::BasUnitDivByModifier(modifier_unit)),
            0b10 => Some(ModifierUnit::BaseUnitMulByModifier(modifier_unit)),
            0b11 => None,
            _ => unreachable!(),
        };

        let is_percentage = (sensor_units_1 & 0x1) == 0x1;

        Self {
            rate,
            modifier,
            base_unit,
            is_percentage,
        }
    }
}

#[derive(Debug, Clone, Copy)]

pub enum Linearization {
    Linear,
    Ln,
    Log10,
    Log2,
    E,
    Exp10,
    Exp2,
    OneOverX,
    Sqr,
    Cube,
    Sqrt,
    CubeRoot,
    Oem(u8),
    Unknown(u8),
}

impl From<u8> for Linearization {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Linear,
            1 => Self::Ln,
            2 => Self::Log10,
            3 => Self::Log2,
            4 => Self::E,
            5 => Self::Exp10,
            6 => Self::Exp2,
            7 => Self::OneOverX,
            8 => Self::Sqr,
            9 => Self::Sqrt,
            10 => Self::Cube,
            11 => Self::Sqrt,
            12 => Self::CubeRoot,
            0x71..=0x7F => Self::Oem(value),
            v => Self::Unknown(v),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Direction {
    UnspecifiedNotApplicable,
    Input,
    Output,
}

impl TryFrom<u8> for Direction {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let dir = match value {
            0b00 => Self::UnspecifiedNotApplicable,
            0b01 => Self::Input,
            0b10 => Self::Output,
            _ => return Err(()),
        };
        Ok(dir)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeLengthRaw<'a>(u8, &'a [u8]);

impl<'a> TypeLengthRaw<'a> {
    pub fn new(value: u8, other_data: &'a [u8]) -> Self {
        Self(value, other_data)
    }
}

impl<'a> From<TypeLengthRaw<'a>> for SensorId {
    fn from(value: TypeLengthRaw<'a>) -> Self {
        let TypeLengthRaw(value, data) = value;
        let type_code = (value >> 6) & 0x3;

        let length = value & 0x1F;

        let data = &data[..(length as usize).min(data.len())];

        let str = core::str::from_utf8(data).map(ToString::to_string);

        match type_code {
            0b00 => SensorId::Unicode(str.unwrap()),
            0b01 => SensorId::BCDPlus(data.to_vec()),
            0b10 => SensorId::Ascii6BPacked(data.to_vec()),
            0b11 => SensorId::Ascii8BAndLatin1(str.unwrap()),
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SensorId {
    Unicode(String),
    BCDPlus(Vec<u8>),
    Ascii6BPacked(Vec<u8>),
    Ascii8BAndLatin1(String),
}

impl core::fmt::Display for SensorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SensorId::Unicode(v) => write!(f, "{}", v),
            SensorId::Ascii8BAndLatin1(v) => write!(f, "{}", v),
            _ => todo!(),
        }
    }
}

impl Default for SensorId {
    fn default() -> Self {
        Self::Unicode("".into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SensorNumber(pub NonMaxU8);

impl SensorNumber {
    pub fn new(value: NonMaxU8) -> Self {
        Self(value)
    }

    pub fn get(&self) -> u8 {
        self.0.get()
    }
}

#[derive(Debug, Clone)]
pub struct RecordHeader {
    pub id: RecordId,

    pub sdr_version_major: u8,
    pub sdr_version_minor: u8,
}

#[derive(Debug, Clone)]
pub struct Record {
    pub header: RecordHeader,
    pub contents: RecordContents,
}

#[derive(Debug, Clone)]
pub enum RecordContents {
    FullSensor(FullSensorRecord),
    CompactSensor(CompactSensorRecord),
    FruDeviceLocator(FruDeviceLocator),
    Unknown { ty: u8, data: Vec<u8> },
}

impl Record {
    pub fn common_data(&self) -> Option<&SensorRecordCommon> {
        match &self.contents {
            RecordContents::FullSensor(s) => Some(s.common()),
            RecordContents::CompactSensor(s) => Some(s.common()),
            RecordContents::FruDeviceLocator(_) => None,
            RecordContents::Unknown { .. } => None,
        }
    }

    pub fn full_sensor(&self) -> Option<&FullSensorRecord> {
        if let RecordContents::FullSensor(full_sensor) = &self.contents {
            Some(full_sensor)
        } else {
            None
        }
    }

    pub fn compact_sensor(&self) -> Option<&CompactSensorRecord> {
        if let RecordContents::CompactSensor(compact_sensor) = &self.contents {
            Some(compact_sensor)
        } else {
            None
        }
    }

    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 5 {
            return None;
        }

        let record_id = RecordId::new_raw(u16::from_le_bytes([data[0], data[1]]));
        let sdr_version_min = (data[2] & 0xF0) >> 4;
        let sdr_version_maj = data[2] & 0x0F;
        let record_type = data[3];
        let record_length = data[4];

        let record_data = &data[5..];
        if record_data.len() != record_length as usize {
            return None;
        }

        let contents = if record_type == 0x01 {
            RecordContents::FullSensor(FullSensorRecord::parse(record_data).ok()?)
        } else if record_type == 0x02 {
            RecordContents::CompactSensor(CompactSensorRecord::parse(record_data)?)
        } else if record_type == 0x11 {
            RecordContents::FruDeviceLocator(FruDeviceLocator::parse(record_data)?)
        } else {
            RecordContents::Unknown {
                ty: record_type,
                data: record_data.to_vec(),
            }
        };

        Some(Self {
            header: RecordHeader {
                id: record_id,
                sdr_version_minor: sdr_version_min,
                sdr_version_major: sdr_version_maj,
            },
            contents,
        })
    }

    pub fn id(&self) -> Option<&SensorId> {
        match &self.contents {
            RecordContents::FullSensor(full) => Some(full.id_string()),
            RecordContents::CompactSensor(compact) => Some(compact.id_string()),
            RecordContents::FruDeviceLocator(fru) => Some(&fru.id_string),
            RecordContents::Unknown { .. } => None,
        }
    }

    pub fn sensor_number(&self) -> Option<SensorNumber> {
        match &self.contents {
            RecordContents::FullSensor(full) => Some(full.sensor_number()),
            RecordContents::CompactSensor(compact) => Some(compact.sensor_number()),
            RecordContents::FruDeviceLocator(_) => None,
            RecordContents::Unknown { .. } => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SensorRecordCommon {
    pub key: SensorKey,
    // TODO: make a type EntityId
    pub entity_id: u8,
    pub entity_instance: EntityInstance,
    pub initialization: SensorInitialization,
    pub capabilities: SensorCapabilities,
    pub ty: SensorType,
    pub event_reading_type_code: EventReadingTypeCodes,
    pub sensor_units: SensorUnits,
    pub sensor_id: SensorId,
}

impl SensorRecordCommon {
    /// Parse common sensor record data, but set the SensorID to an empty UTF-8 String.
    ///
    /// You _must_ remember to [`SensorRecordCommon::set_id`] once the ID of the
    /// record has been parsed.
    pub(crate) fn parse_without_id(record_data: &[u8]) -> Option<(Self, &[u8])> {
        if record_data.len() < 17 {
            return None;
        }

        let sensor_key = SensorKey::parse(&record_data[..3])?;

        let entity_id = record_data[3];

        let entity_instance = record_data[4];
        let entity_instance = EntityInstance::from(entity_instance);

        let initialization = record_data[5];
        let initialization = SensorInitialization::from(initialization);

        let sensor_capabilities = record_data[6];

        let sensor_type = record_data[7].into();
        let event_reading_type_code = record_data[8].into();

        let assertion_event_mask_lower_thrsd_reading_mask =
            u16::from_le_bytes([record_data[9], record_data[10]]);
        let deassertion_event_mask_upper_thrsd_reading_mask =
            u16::from_le_bytes([record_data[11], record_data[12]]);
        let settable_thrsd_readable_thrsd_mask =
            u16::from_le_bytes([record_data[13], record_data[14]]);

        let capabilities = SensorCapabilities::new(
            sensor_capabilities,
            assertion_event_mask_lower_thrsd_reading_mask,
            deassertion_event_mask_upper_thrsd_reading_mask,
            settable_thrsd_readable_thrsd_mask,
        );

        let sensor_units_1 = record_data[15];
        let base_unit = record_data[16];
        let modifier_unit = record_data[17];

        let sensor_units = SensorUnits::from(sensor_units_1, base_unit, modifier_unit);

        Some((
            Self {
                key: sensor_key,
                entity_id,
                entity_instance,
                initialization,
                capabilities,
                ty: sensor_type,
                event_reading_type_code,
                sensor_units,
                sensor_id: Default::default(),
            },
            &record_data[18..],
        ))
    }

    pub(crate) fn set_id(&mut self, id: SensorId) {
        self.sensor_id = id;
    }
}

impl Loggable for Record {
    fn into_log(&self) -> Vec<crate::fmt::LogItem> {
        let full = self.full_sensor();
        let compact = self.compact_sensor();

        let mut log = Vec::new();

        if full.is_some() {
            log.push((0, "SDR Record (Full)").into());
        } else if compact.is_some() {
            log.push((0, "SDR Record (Compact)").into());
        } else {
            log.push((0, "Cannot log unknown sensor type").into());
            return log;
        }

        let RecordHeader {
            id,
            sdr_version_major: sdr_v_maj,
            sdr_version_minor: sdr_v_min,
        } = &self.header;

        log.push((1, "Record ID", format!("0x{:04X}", id.0)).into());
        log.push((1, "SDR Version", format!("{sdr_v_maj}.{sdr_v_min}")).into());

        if let Some(common) = self.common_data() {
            log.push((1, "Sensor Type", format!("{:?}", common.ty)).into());
        }

        if let Some(full) = full {
            full.key_data().log_into(1, &mut log);

            let display = |v: Value| v.display(true);

            let nominal_reading = full
                .nominal_value()
                .map(display)
                .unwrap_or("Unknown".into());

            let max_reading = full.max_reading().map(display).unwrap_or("Unknown".into());
            let min_reading = full.min_reading().map(display).unwrap_or("Unknown".into());

            log.push((1, "Sensor ID", full.id_string()).into());
            log.push((1, "Entity ID", full.entity_id()).into());
            log.push((1, "Nominal reading", nominal_reading).into());
            log.push((1, "Max reading", max_reading).into());
            log.push((1, "Min reading", min_reading).into());
        } else if let Some(compact) = compact {
            compact.key_data().log_into(1, &mut log);
            log.push((1, "Sensor ID", compact.id_string()).into());
        }

        log
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sensor_owner_round_trip() {
        for x in 0u8..=255u8 {
            let o = SensorOwner::from(x);
            let value: u8 = o.into();
            assert_eq!(x, value);
        }
    }
}
