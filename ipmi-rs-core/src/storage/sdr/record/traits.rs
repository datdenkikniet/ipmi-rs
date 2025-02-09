use crate::storage::sdr::{event_reading_type_code::EventReadingTypeCodes, SensorType};

use super::{
    Direction, EntityInstance, SensorCapabilities, SensorId, SensorKey, SensorNumber,
    SensorRecordCommon,
};

pub trait DirectionalSensor {
    fn direction(&self) -> &Direction;
}

pub trait IdentifiableSensor {
    fn id_string(&self) -> &SensorId;

    fn entity_id(&self) -> u8;
}

pub trait InstancedSensor: IdentifiableSensor {
    fn ty(&self) -> &SensorType;

    fn event_reading_type_codes(&self) -> &EventReadingTypeCodes;

    fn entity_instance(&self) -> &EntityInstance;

    fn key_data(&self) -> &SensorKey;
}

pub trait WithSensorRecordCommon {
    fn common(&self) -> &SensorRecordCommon;

    fn capabilities(&self) -> &SensorCapabilities {
        &self.common().capabilities
    }

    fn sensor_number(&self) -> SensorNumber {
        self.common().key.sensor_number
    }
}

impl<T> IdentifiableSensor for T
where
    T: WithSensorRecordCommon,
{
    fn id_string(&self) -> &SensorId {
        &self.common().sensor_id
    }

    fn entity_id(&self) -> u8 {
        self.common().entity_id
    }
}

impl<T> InstancedSensor for T
where
    T: WithSensorRecordCommon,
{
    fn ty(&self) -> &SensorType {
        &self.common().ty
    }

    fn entity_instance(&self) -> &EntityInstance {
        &self.common().entity_instance
    }

    fn key_data(&self) -> &SensorKey {
        &self.common().key
    }

    fn event_reading_type_codes(&self) -> &EventReadingTypeCodes {
        &self.common().event_reading_type_code
    }
}
