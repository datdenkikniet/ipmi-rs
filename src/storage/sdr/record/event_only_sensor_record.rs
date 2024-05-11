use crate::storage::sdr::event_reading_type_code::EventReadingTypeCodes;
use crate::storage::sdr::record::compact_sensor_record::{IdStringModifier, RecordSharing};
use crate::storage::sdr::record::{Direction, EntityInstance, SensorId, SensorKey, TypeLengthRaw};
use crate::storage::sdr::SensorType;

use super::ParseError;

#[derive(Debug, Clone)]

pub struct EventOnlySensorRecord {
    pub key: SensorKey,
    pub entity_id: u8,
    pub entity_instance: EntityInstance,
    pub ty: SensorType,
    pub event_reading_type_code: EventReadingTypeCodes,
    pub direction: Direction,
    pub record_sharing: RecordSharing,
    pub oem_reserved: u8,
    pub id_string: SensorId,
}

impl EventOnlySensorRecord {
    pub fn parse(record_data: &[u8]) -> Result<Self, ParseError> {
        let key = SensorKey::parse(&record_data[..3])?;

        let entity_id = record_data[3];
        let entity_instance = EntityInstance::from(record_data[4]);
        let ty = record_data[5].into();
        let event_reading_type_code = record_data[6].into();

        let direction_sharing_1 = record_data[7];
        let direction_sharing_2 = record_data[8];

        let direction = Direction::try_from((direction_sharing_1 & 0xC) >> 6)?;
        let id_string_instance_modifier = match (direction_sharing_1 & 0x30) >> 4 {
            0b00 => Ok(IdStringModifier::Numeric),
            0b01 => Ok(IdStringModifier::Alpha),
            v => Err(ParseError::InvalidIdStringModifier(v)),
        }?;

        let share_count = direction_sharing_1 & 0xF;
        let entity_instance_increments = (direction_sharing_2 & 0x80) == 0x80;
        let modifier_offset = direction_sharing_2 & 0x3F;

        let record_sharing = RecordSharing {
            id_string_modifier: id_string_instance_modifier,
            share_count,
            entity_instance_increments,
            modifier_offset,
        };

        // one reserved byte
        let oem_reserved = record_data[10];
        let id_string_type_len = record_data[11];
        let id_string_bytes = &record_data[12..];

        let id_string = TypeLengthRaw::new(id_string_type_len, id_string_bytes).try_into()?;

        Ok(EventOnlySensorRecord {
            key,
            entity_id,
            entity_instance,
            ty,
            event_reading_type_code,
            direction,
            record_sharing,
            oem_reserved,
            id_string,
        })
    }
}
