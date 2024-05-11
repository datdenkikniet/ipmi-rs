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
    common: SensorRecordCommon,
    pub direction: Direction,
    pub record_sharing: RecordSharing,
    pub positive_going_threshold_hysteresis_value: u8,
    pub negative_going_threshold_hysteresis_value: u8,
    pub oem_data: u8,
}

impl SensorRecord for CompactSensorRecord {
    fn common(&self) -> &SensorRecordCommon {
        &self.common
    }

    fn direction(&self) -> Direction {
        self.direction
    }
}

impl CompactSensorRecord {
    pub fn parse(record_data: &[u8]) -> Result<Self, ParseError> {
        if record_data.len() < 26 {
            return Err(ParseError::NotEnoughData);
        }

        let (mut common, record_data) = SensorRecordCommon::parse_without_id(record_data)?;

        let direction_sharing_1 = record_data[0];
        let direction_sharing_2 = record_data[1];

        let direction = Direction::try_from((direction_sharing_1 & 0xC) >> 6)?;
        let id_string_instance_modifier = match (direction_sharing_1 & 0x30) >> 4 {
            0b00 => IdStringModifier::Numeric,
            0b01 => IdStringModifier::Alpha,
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
        let id_string = TypeLengthRaw::new(id_string_type_len, id_string_bytes).try_into()?;

        common.set_id(id_string);

        Ok(Self {
            common,
            direction,
            record_sharing,
            positive_going_threshold_hysteresis_value,
            negative_going_threshold_hysteresis_value,
            oem_data,
        })
    }
}
