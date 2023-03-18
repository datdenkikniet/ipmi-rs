use super::*;

#[derive(Debug, Clone)]

pub struct FullSensorRecord {
    pub owner: SensorOwner,
    pub owner_channel: u8,
    pub owner_lun: LogicalUnit,
    pub sensor_number: NonMaxU8,
    pub entity_id: u8,
    pub entity_instance: EntityInstance,
    pub initialization: SensorInitialization,
    pub capabilities: SensorCapabilities,
    pub ty: u8,
    pub event_reading_type_code: u8,
    pub sensor_units: SensorUnits,
    pub base_unit: Unit,
    pub modifier_unit: Option<Unit>,
    pub linearization: Linearization,
    pub m: i16,
    pub tolerance: u8,
    pub b: i16,
    pub accuracy: u16,
    pub accuracy_exponent: u8,
    pub direction: Option<Direction>,
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
    pub positive_going_threshold_hysteresis_value: u8,
    pub negative_going_threshold_hysteresis_value: u8,
    pub oem_data: u8,
    pub id_string: SensorId,
}

impl FullSensorRecord {
    pub fn parse(record_data: &[u8]) -> Option<Self> {
        if record_data.len() < 43 {
            return None;
        }

        let owner = SensorOwner::from(record_data[0]);
        let owner_lun_channel = record_data[1];
        let owner_channel = (owner_lun_channel & 0xF0) >> 4;
        let owner_lun = LogicalUnit::try_from(owner_lun_channel & 0x3).unwrap();

        let sensor_number = NonMaxU8::new(record_data[2])?;

        let entity_id = record_data[3];

        let entity_instance = record_data[4];
        let entity_instance = EntityInstance::from(entity_instance);

        let initialization = record_data[5];
        let initialization = SensorInitialization::from(initialization);

        let sensor_capabilities = record_data[6];

        let sensor_type = record_data[7];
        let event_reading_type_code = record_data[8];

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
        let sensor_units = SensorUnits::from(sensor_units_1);

        let base_unit = record_data[16];
        let base_unit = Unit::try_from(base_unit).unwrap_or(Unit::Unknown);

        let modifier_unit = record_data[17];
        let modifier_unit = if modifier_unit == 0 {
            None
        } else {
            Some(Unit::try_from(base_unit).unwrap_or(Unit::Unknown))
        };

        let linearization = record_data[18];
        let linearization = Linearization::from(linearization & 0x7F);

        let m_lsb = record_data[19];
        let m_msb_tolerance = record_data[20];
        let m_sign = m_msb_tolerance & 0x80;
        let m = i16::from_le_bytes([m_lsb, m_sign | (m_msb_tolerance >> 6) & 0x1]);

        let tolerance = m_msb_tolerance & 0x3F;

        let b_lsb = record_data[21];
        let b_msb_accuracy_lsb = record_data[22];

        let b_sign = b_msb_accuracy_lsb & 1;
        let b = i16::from_le_bytes([b_lsb, b_sign | (b_msb_accuracy_lsb >> 6)]);

        let accuracy_msb_accuracy_exp_sensor_dir = record_data[23];

        let accuracy = u16::from_le_bytes([
            (accuracy_msb_accuracy_exp_sensor_dir >> 4) & 0xF,
            (b_msb_accuracy_lsb & 0x3F),
        ]);

        let accuracy_exponent = (accuracy_msb_accuracy_exp_sensor_dir >> 2) & 0x3;

        let direction = Direction::try_from(accuracy_msb_accuracy_exp_sensor_dir & 0b11).ok();

        let r_exp_b_exp = record_data[24];

        let r_sign = r_exp_b_exp & 1;
        let result_exponent = (r_sign | ((r_exp_b_exp >> 4) & 0x3)) as i8;

        let b_sign = (r_exp_b_exp & 0x08) << 4;
        let b_exponent = (b_sign | (r_exp_b_exp & 0x3)) as i8;

        let analog_characteristics = record_data[25];

        let nominal_reading = record_data[26];
        let nominal_reading = if (analog_characteristics & 0x1) == 0x1 {
            Some(nominal_reading)
        } else {
            None
        };

        let normal_maximum = record_data[27];
        let normal_maximum = if (analog_characteristics & 0x2) == 0x2 {
            Some(normal_maximum)
        } else {
            None
        };

        let normal_minimum = record_data[28];
        let normal_minimum = if (analog_characteristics & 0x4) == 0x4 {
            Some(normal_minimum)
        } else {
            None
        };

        let max_reading = record_data[29];
        let min_reading = record_data[30];

        let upper_non_recoverable_threshold = record_data[31];
        let upper_critical_threshold = record_data[32];
        let upper_non_critical_threshold = record_data[33];
        let lower_non_recoverable_threshold = record_data[34];
        let lower_critical_threshold = record_data[35];
        let lower_non_critical_threshold = record_data[36];
        let positive_going_threshold_hysteresis_value = record_data[37];
        let negative_going_threshold_hysteresis_value = record_data[38];

        // Two reserved bytes in between

        let oem_data = record_data[41];

        let id_string_type_len = record_data[42];
        let id_string_bytes = &record_data[43..];
        let id_string = TypeLengthRaw::new(id_string_type_len, id_string_bytes).into();

        Some(Self {
            owner,
            owner_channel,
            owner_lun,
            sensor_number,
            entity_id,
            entity_instance,
            initialization,
            capabilities,
            ty: sensor_type,
            event_reading_type_code,
            sensor_units,
            base_unit,
            modifier_unit,
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
}
