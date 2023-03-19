use super::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IdStringModifier {
    Numeric,
    Alpha,
}

#[derive(Debug, Clone)]
pub struct RecordSharing {
    pub id_string_modifier: IdStringModifier,
    pub share_count: u8,
    pub entity_instance_increments: bool,
    pub modifier_offset: u8,
}

#[derive(Debug, Clone)]
pub struct CompactSensorRecord {
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
    pub direction: Direction,
    pub record_sharing: RecordSharing,
    pub positive_going_threshold_hysteresis_value: u8,
    pub negative_going_threshold_hysteresis_value: u8,
    pub oem_data: u8,
    pub id_string: SensorId,
}

impl CompactSensorRecord {
    pub fn parse(record_data: &[u8]) -> Option<Self> {
        if record_data.len() < 26 {
            return None;
        }

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
        ) = SensorRecordCommon::parse(record_data)?;

        let direction_sharing_1 = record_data[0];
        let direction_sharing_2 = record_data[1];

        let direction = Direction::try_from((direction_sharing_1 & 0xC) >> 6).unwrap();
        let id_string_instance_modifier = match (direction_sharing_1 & 0x30) >> 4 {
            0b00 => IdStringModifier::Numeric,
            0b01 => IdStringModifier::Numeric,
            _ => panic!("Invalid ID string modifier, no fallback available."),
        };

        let share_count = direction_sharing_1 & 0xF;
        let entity_instance_increments = (direction_sharing_2 & 0x80) == 0x80;
        let modifier_offset = direction_sharing_2 & 0x3F;

        let record_sharing = RecordSharing {
            id_string_modifier: id_string_instance_modifier,
            share_count,
            entity_instance_increments,
            modifier_offset,
        };

        let positive_going_threshold_hysteresis_value = record_data[2];
        let negative_going_threshold_hysteresis_value = record_data[3];

        // Three reserved bytes

        let oem_data = record_data[7];

        let id_string_type_len = record_data[8];
        let id_string_bytes = &record_data[9..];
        let id_string = TypeLengthRaw::new(id_string_type_len, id_string_bytes).into();

        Some(Self {
            key,
            entity_id,
            entity_instance,
            initialization,
            capabilities,
            ty,
            event_reading_type_code,
            sensor_units,
            direction,
            record_sharing,
            positive_going_threshold_hysteresis_value,
            negative_going_threshold_hysteresis_value,
            oem_data,
            id_string,
        })
    }
}
