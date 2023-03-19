use std::num::NonZeroU8;

use super::*;

#[derive(Debug, Clone)]

pub struct FullSensorRecord {
    pub key: SensorKey,
    // TODO: make a type EntityId
    pub entity_id: u8,
    pub entity_instance: EntityInstance,
    pub initialization: SensorInitialization,
    pub capabilities: SensorCapabilities,
    pub ty: SensorType,
    // TODO: Make a type EventReadingTypeCode
    pub event_reading_type_code: u8,
    pub sensor_units: SensorUnits,
    pub analog_data_format: Option<DataFormat>,
    pub linearization: Linearization,
    pub m: i16,
    pub tolerance: u8,
    pub b: i16,
    pub accuracy: u16,
    pub accuracy_exponent: u8,
    pub direction: Direction,
    pub result_exponent: i8,
    pub b_exponent: i8,
    // TODO: convert these to the correct
    // units based on sensor_units
    pub nominal_reading: Option<u8>,
    pub normal_maximum: Option<u8>,
    pub normal_minimum: Option<u8>,
    pub max_reading: u8,
    pub min_reading: u8,
    pub upper_non_recoverable_threshold: u8,
    pub upper_critical_threshold: u8,
    pub upper_non_critical_threshold: u8,
    pub lower_non_recoverable_threshold: u8,
    pub lower_critical_threshold: u8,
    pub lower_non_critical_threshold: u8,
    pub positive_going_threshold_hysteresis_value: Option<NonZeroU8>,
    pub negative_going_threshold_hysteresis_value: Option<NonZeroU8>,
    pub oem_data: u8,
    pub id_string: SensorId,
}

#[derive(Debug, Clone, Copy)]
pub enum ParseFullSensorRecordError {
    NotEnoughData,
    CouldNotParseCommon,
    NotEnoughDataAfterCommon,
}

impl FullSensorRecord {
    pub fn parse(record_data: &[u8]) -> Result<Self, ParseFullSensorRecordError> {
        use ParseFullSensorRecordError::*;

        if record_data.len() < 15 {
            return Err(NotEnoughData);
        }

        let sensor_units_1 = record_data[15];

        let analog_data_format = match (sensor_units_1 >> 6) & 0x03 {
            0b00 => Some(DataFormat::Unsigned),
            0b01 => Some(DataFormat::OnesComplement),
            0b10 => Some(DataFormat::TwosComplement),
            0b11 => None,
            _ => unreachable!(),
        };

        let (
            SensorRecordCommon {
                key,
                entity_id,
                entity_instance,
                initialization,
                capabilities,
                ty,
                event_reading_type_code,
                sensor_units,
            },
            record_data,
        ) = SensorRecordCommon::parse(record_data).ok_or(CouldNotParseCommon)?;

        if record_data.len() < 24 {
            return Err(NotEnoughDataAfterCommon);
        }

        let linearization = record_data[0];
        let linearization = Linearization::from(linearization & 0x7F);

        let m_lsb = record_data[1];
        let m_msb_tolerance = record_data[2];
        let m_sign = m_msb_tolerance & 0x80;

        let m = i16::from_le_bytes([m_lsb, m_sign | (m_msb_tolerance >> 6) & 0x1]);

        let tolerance = m_msb_tolerance & 0x3F;

        let b_lsb = record_data[3];
        let b_msb_accuracy_lsb = record_data[4];

        let b_sign = b_msb_accuracy_lsb & 1;
        let b = i16::from_le_bytes([b_lsb, b_sign | (b_msb_accuracy_lsb >> 6)]);

        let accuracy_msb_accuracy_exp_sensor_dir = record_data[5];

        let accuracy = u16::from_le_bytes([
            (accuracy_msb_accuracy_exp_sensor_dir >> 4) & 0xF,
            (b_msb_accuracy_lsb & 0x3F),
        ]);

        let accuracy_exponent = (accuracy_msb_accuracy_exp_sensor_dir >> 2) & 0x3;

        let direction = Direction::try_from(accuracy_msb_accuracy_exp_sensor_dir & 0b11)
            .unwrap_or(Direction::UnspecifiedNotApplicable);

        let r_exp_b_exp = record_data[6];

        let r_sign = r_exp_b_exp & 1;
        let result_exponent = (r_sign | ((r_exp_b_exp >> 4) & 0x3)) as i8;

        let b_sign = (r_exp_b_exp & 0x08) << 4;
        let b_exponent = (b_sign | (r_exp_b_exp & 0x3)) as i8;

        let analog_characteristics = record_data[7];

        let nominal_reading = record_data[8];
        let nominal_reading = if (analog_characteristics & 0x1) == 0x1 {
            Some(nominal_reading)
        } else {
            None
        };

        let normal_maximum = record_data[9];
        let normal_maximum = if (analog_characteristics & 0x2) == 0x2 {
            Some(normal_maximum)
        } else {
            None
        };

        let normal_minimum = record_data[10];
        let normal_minimum = if (analog_characteristics & 0x4) == 0x4 {
            Some(normal_minimum)
        } else {
            None
        };

        let max_reading = record_data[11];
        let min_reading = record_data[12];

        let upper_non_recoverable_threshold = record_data[13];
        let upper_critical_threshold = record_data[14];
        let upper_non_critical_threshold = record_data[15];
        let lower_non_recoverable_threshold = record_data[16];
        let lower_critical_threshold = record_data[17];
        let lower_non_critical_threshold = record_data[18];
        let positive_going_threshold_hysteresis_value = NonZeroU8::new(record_data[19]);
        let negative_going_threshold_hysteresis_value = NonZeroU8::new(record_data[20]);

        // Two reserved bytes in between

        let oem_data = record_data[23];

        let id_string_type_len = record_data[24];
        let id_string_bytes = &record_data[25..];

        let id_string = TypeLengthRaw::new(id_string_type_len, id_string_bytes).into();

        Ok(Self {
            key,
            entity_id,
            entity_instance,
            initialization,
            capabilities,
            ty,
            event_reading_type_code,
            analog_data_format,
            sensor_units,
            linearization,
            m,
            tolerance,
            b,
            accuracy,
            accuracy_exponent,
            direction,
            result_exponent,
            b_exponent,
            nominal_reading,
            normal_maximum,
            normal_minimum,
            max_reading,
            min_reading,
            upper_non_recoverable_threshold,
            upper_critical_threshold,
            upper_non_critical_threshold,
            lower_non_recoverable_threshold,
            lower_critical_threshold,
            lower_non_critical_threshold,
            positive_going_threshold_hysteresis_value,
            negative_going_threshold_hysteresis_value,
            oem_data,
            id_string,
        })
    }

    pub fn threshold(&self, kind: ThresholdKind) -> Threshold {
        let readable = self.capabilities.threshold_access.readable(kind);
        let settable = self.capabilities.threshold_access.settable(kind);

        let asserts = self.capabilities.assertion_threshold_events.for_kind(kind);
        let deasserts = self
            .capabilities
            .deassertion_threshold_events
            .for_kind(kind);

        Threshold {
            kind,
            readable,
            settable,
            event_assert_going_high: asserts.contains(&EventKind::GoingHigh),
            event_assert_going_low: asserts.contains(&EventKind::GoingLow),
            event_deassert_going_high: deasserts.contains(&EventKind::GoingHigh),
            event_deassert_going_low: deasserts.contains(&EventKind::GoingLow),
        }
    }

    fn convert(&self, value: u8) -> Option<f32> {
        let m = self.m as f32;
        let b = (self.b as f32).powf(self.b_exponent as f32);
        let format = self.analog_data_format?;

        let value = match format {
            DataFormat::Unsigned => value as f32,
            DataFormat::OnesComplement => !value as i8 as f32,
            DataFormat::TwosComplement => value as i8 as f32,
        };

        Some(((m * value) + b) / m)
    }

    pub fn nominal_value(&self) -> Option<f32> {
        self.convert(self.nominal_reading?)
    }

    pub fn normal_max(&self) -> Option<f32> {
        self.convert(self.normal_maximum?)
    }

    pub fn normal_min(&self) -> Option<f32> {
        self.convert(self.normal_minimum?)
    }

    pub fn positive_going_hysteresis(&self) -> Option<f32> {
        let value = self.positive_going_threshold_hysteresis_value?;
        self.convert(value.get())
    }

    pub fn negative_going_threshold_hysteresis(&self) -> Option<f32> {
        let value = self.negative_going_threshold_hysteresis_value?;
        self.convert(value.get())
    }
}
