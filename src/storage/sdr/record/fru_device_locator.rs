use crate::connection::LogicalUnit;
use crate::storage::sdr::record::{SensorId, TypeLengthRaw};

#[derive(Debug, Clone)]
pub struct LogicalFruDevice {
    pub fru_device_id: u8,
}

#[derive(Debug, Clone)]
pub struct PhysicalFruDevice {
    pub i2c_address: u8,
}

#[derive(Debug, Clone)]
pub enum FruDevice {
    Logical(LogicalFruDevice),
    Physical(PhysicalFruDevice),
}

#[derive(Debug, Clone)]
pub struct FruRecordKey {
    pub device_access_address: u8,
    pub fru_device: FruDevice,
    pub lun: LogicalUnit,
    pub private_bus_id: u8,
    pub channel_number: u8,
}

#[derive(Debug, Clone)]
pub struct FruDeviceLocator {
    pub record_key: FruRecordKey,
    pub device_type: u8,
    pub device_type_modifier: u8,
    pub fru_entity_id: u8,
    pub fru_entity_instance: u8,
    pub oem_reserved: u8,
    pub id_string: SensorId,
}

impl FruDeviceLocator {
    pub fn parse(record_data: &[u8]) -> Option<Self> {
        if record_data.len() < 8 {
            return None;
        }

        let device_access_address = record_data[0] >> 1;

        let fru_device = if record_data[1] & 0x80 == 0x80 {
            FruDevice::Physical(PhysicalFruDevice {
                i2c_address: (record_data[1] >> 1),
            })
        } else {
            FruDevice::Logical(LogicalFruDevice {
                fru_device_id: record_data[1],
            })
        };

        let lun = LogicalUnit::try_from((record_data[2] >> 3) & 0b11).ok()?;
        let private_bus_id = record_data[2] & 0b111;
        let channel_number = record_data[3];

        let record_key = FruRecordKey {
            device_access_address,
            fru_device,
            lun,
            private_bus_id,
            channel_number,
        };

        let device_type = record_data[5];
        let device_type_modifier = record_data[6];
        let fru_entity_id = record_data[7];
        let fru_entity_instance = record_data[8];
        let oem_reserved = record_data[9];

        let id_string_type_len = record_data[10];
        let id_string_bytes = &record_data[11..];

        let id_string = TypeLengthRaw::new(id_string_type_len, id_string_bytes).into();

        Some(Self {
            record_key,
            device_type,
            device_type_modifier,
            fru_entity_id,
            fru_entity_instance,
            oem_reserved,
            id_string,
        })
    }

    pub fn id_string(&self) -> &SensorId {
        &self.id_string
    }
}
