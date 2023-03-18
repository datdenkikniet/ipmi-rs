use std::num::{NonZeroU16, NonZeroU8};

use nonmax::NonMaxU8;

use crate::{
    connection::{IpmiCommand, LogicalUnit, Message, NetFn, ParseResponseError},
    storage::Unit,
};

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
pub struct InitState {
    pub event_generation_enabled: bool,
    pub sensor_scanning_enabled: bool,
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

#[derive(Debug, Clone, Copy)]

pub struct Thresholds {
    pub lower_non_recoverable: bool,
    pub lower_critical: bool,
    pub lower_non_critical: bool,
    pub upper_non_recoverable: bool,
    pub upper_critical: bool,
    pub upper_non_critical: bool,
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

#[derive(Debug, Clone, Copy)]
pub struct SensorCapabilities {
    pub ignore: bool,
    pub auto_rearm: bool,
    pub hysteresis: HysteresisCapability,
    pub threshold_access: ThresholdAccessCapability,
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
            lower_non_critical: ((discrete_rd_thrsd_set_thrshd_read >> 0) & 0x1) == 1,
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
            threshold_access: threshold_access_support,
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
    BasicUnitDivByModifierUnit,
    BasicUnitMulByModifierUnit,
}

#[derive(Debug, Clone, Copy)]
pub struct SensorUnits {
    pub analog_data_format: Option<DataFormat>,
    pub rate: Option<RateUnit>,
    pub modifier: Option<ModifierUnit>,
    pub is_percentage: bool,
}

impl From<u8> for SensorUnits {
    fn from(sensor_units_1: u8) -> Self {
        let analog_data_format = match (sensor_units_1 >> 6) & 0x03 {
            0b00 => Some(DataFormat::Unsigned),
            0b01 => Some(DataFormat::OnesComplement),
            0b10 => Some(DataFormat::TwosComplement),
            0b11 => None,
            _ => unreachable!(),
        };

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

        let modifier = match (sensor_units_1 >> 1) & 0b11 {
            0b00 => None,
            0b01 => Some(ModifierUnit::BasicUnitDivByModifierUnit),
            0b10 => Some(ModifierUnit::BasicUnitMulByModifierUnit),
            0b11 => None,
            _ => unreachable!(),
        };

        let is_percentage = (sensor_units_1 & 0x1) == 0x1;

        Self {
            analog_data_format,
            rate,
            modifier,
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
    InverseCube,
    Oem(u8),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeLengthRaw<'a>(u8, &'a [u8]);

impl<'a> TypeLengthRaw<'a> {
    pub fn new(value: u8, other_data: &'a [u8]) -> Self {
        Self(value, other_data)
    }
}

impl<'a> Into<SensorId> for TypeLengthRaw<'a> {
    fn into(self) -> SensorId {
        let Self(value, data) = self;
        let type_code = (value >> 6) & 0x3;

        match type_code {
            0b00 => {
                let str = core::str::from_utf8(data).unwrap().to_string();
                SensorId::Unicode(str)
            }
            0b01 => SensorId::BCDPlus(data.to_vec()),
            0b10 => SensorId::Ascii6BPacked(data.to_vec()),
            0b11 => SensorId::Ascii8BAndLatin1(data.to_vec()),
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum SensorId {
    Unicode(String),
    BCDPlus(Vec<u8>),
    Ascii6BPacked(Vec<u8>),
    Ascii8BAndLatin1(Vec<u8>),
}

impl SensorId {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            SensorId::Unicode(v) => Some(v.as_str()),
            _ => None,
        }
    }
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
        capabilities: SensorCapabilities,
        ty: u8,
        event_reading_type_code: u8,
        threshold_reading_mask: Thresholds,
        assertion_event_mask: ThresholdAssertEventMask,
        deassertion_event_mask: ThresholdAssertEventMask,
        settable_threshold_mask: Thresholds,
        readable_threshold_msak: Thresholds,
        sensor_units: SensorUnits,
        base_unit: u8,
        modifier_unit: Option<NonZeroU8>,
        linearization: Linearization,
        m: i16,
        tolerance: u8,
        b: i16,
        accuracy: u16,
        accuracy_exponent: u8,
        result_exponent: i8,
        b_exponent: i8,
        // TODO: convert these to the correct
        // units based on sensor_units
        nominal_reading: Option<u8>,
        normal_maximum: Option<u8>,
        normal_minimum: Option<u8>,
        max_reading: u8,
        minimum_reading: u8,
        upper_non_recoverable_threshold: u8,
        upper_critical_threshold: u8,
        upper_non_critical_threshold: u8,
        lower_non_recoverable_threshold: u8,
        lower_critical_threshold: u8,
        lower_non_critical_threshold: u8,
        positive_going_threshold_hystersis_value: u8,
        negative_going_threshold_hystersis_value: u8,
        oem_data: u8,
        id_string: SensorId,
    },
    Unknown {
        data: Vec<u8>,
    },
}

impl Record {
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

        let sensor_owner = SensorOwner::from(data[5]);
        let sensor_owner_lun = data[6];
        let sensor_owner_channel = (sensor_owner_lun & 0xF0) >> 4;
        let sensor_owner_lun = LogicalUnit::try_from(sensor_owner_lun & 0x3).unwrap();

        let sensor_number = data[7];

        let entity_id = data[8];

        let entity_instance = data[9];
        let entity_instance = EntityInstance::from(entity_instance);

        let sensor_initialization = data[10];
        let sensor_initialization = SensorInitialization::from(sensor_initialization);

        let sensor_capabilities = data[11];

        let sensor_type = data[12];
        let event_reading_type_code = data[13];

        let assertion_event_mask_lower_thrsd_reading_mask =
            u16::from_le_bytes([data[14], data[15]]);
        let deassertion_event_mask_upper_thrsd_reading_mask =
            u16::from_le_bytes([data[16], data[17]]);
        let settable_thrsd_readable_thrsd_mask = u16::from_le_bytes([data[18], data[19]]);

        let sensor_capabilities = SensorCapabilities::new(
            sensor_capabilities,
            assertion_event_mask_lower_thrsd_reading_mask,
            deassertion_event_mask_upper_thrsd_reading_mask,
            settable_thrsd_readable_thrsd_mask,
        );

        let sensor_units_1 = data[20];
        let sensor_units = SensorUnits::from(sensor_units_1);

        let base_unit = data[21];
        let unit = Unit::try_from(base_unit).unwrap_or(Unit::Unknown);

        panic!("{:?}", unit);

        let modifier_unit = data[22];
        let linearization = data[23];
        let m_lsb = data[24];
        let m_msb_tolerance = data[25];
        let b_lsb = data[26];
        let b_msb_accuracy_lsb = data[27];
        let accuracy_msb_accuracy_exp_sensor_dir = data[28];
        let r_exp_b_exp = data[29];
        let analog_characteristics = data[30];
        let nominal_reading = data[31];
        let normal_maximum = data[32];
        let normal_minimum = data[33];
        let sensor_max = data[34];
        let sensor_min = data[35];

        let upper_non_recoverabel_threshold = data[36];
        let upper_critical_threshold = data[37];
        let upper_non_critical_threshold = data[38];
        let lower_non_recoverable_threshold = data[39];
        let lower_critical_threshold = data[40];
        let lower_non_critical_threshold = data[41];
        let postitive_going_threshold_hysterisis = data[42];
        let negative_going_threshold_hysterisis = data[43];
        let oem = data[46];
        let id_string_type_len = data[47];
        let id_string_bytes = &data[48..];

        todo!()
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
