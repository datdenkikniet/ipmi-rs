use crate::storage::sdr::record::{SensorId, TypeLengthRaw};

#[derive(Debug, Clone)]
pub struct McRecordKey {
    pub i2c_address: u8,
    pub channel: u8,
}
#[derive(Debug, Clone)]
pub enum GlobalInitialization {
    EnableEventMessageGeneration,
    DisableEventMessageGeneration,
    DoNotInitialize,
    Reserved,
}
#[derive(Debug, Clone)]
pub struct DeviceCapabilities {
    pub chassis_device: bool,
    pub bridge: bool,
    pub ipmi_event_generator: bool,
    pub ipmi_event_receiver: bool,
    pub fru_inventory_device: bool,
    pub sel_device: bool,
    pub sdr_repository_device: bool,
    pub sensor_device: bool,
}
#[derive(Debug, Clone)]
pub struct McDeviceLocatorRecord {
    pub key: McRecordKey,
    pub acpi_system_power_state_notification_required: bool,
    pub acpi_device_power_state_notification_required: bool,
    pub static_controller: bool,

    pub controller_logs_initialization_errors: bool,
    pub log_initialization_errors_accessing_controller: bool,
    pub global_initialization: GlobalInitialization,
    pub device_capabilities: DeviceCapabilities,
    pub entity_id: u8,
    // Note: Unlike entity_instance in sensor SDRs, the IPMI specification specifies this as just
    // an entity instance number, hence not using the `EntityInstance` type here.
    pub entity_instance: u8,
    pub oem_reserved: u8,
    pub id_string: SensorId,
}

impl McDeviceLocatorRecord {
    pub fn parse(record_data: &[u8]) -> Option<Self> {
        let i2c_address = record_data[0] >> 1;
        let channel = record_data[1] & 0b1111;

        let key = McRecordKey {
            i2c_address,
            channel,
        };

        let psn_and_gi = record_data[2];
        let acpi_system_power_state_notification_required =
            (psn_and_gi & 0b1000_0000) == 0b1000_0000;
        let acpi_device_power_state_notification_required =
            (psn_and_gi & 0b0100_0000) == 0b0100_0000;
        let static_controller = (psn_and_gi & 0b0010_0000) == 0b0010_0000;
        // reserved bit
        // should these bools be part of GlobalInitialization?
        let controller_logs_initialization_errors = (record_data[2] & 0b0000_1000) == 0b0000_1000;
        let log_initialization_errors_accessing_controller =
            (record_data[2] & 0b0000_0100) == 0b0000_0100;
        let global_initialization = match record_data[2] & 0b0000_0011 {
            0b00 => GlobalInitialization::EnableEventMessageGeneration,
            0b01 => GlobalInitialization::DisableEventMessageGeneration,
            0b10 => GlobalInitialization::DoNotInitialize,
            0b11 => GlobalInitialization::Reserved,
            _ => unreachable!(),
        };

        let dc_byte = record_data[3];

        let device_capabilities = DeviceCapabilities {
            chassis_device: (dc_byte & 0b1000_0000) == 0b1000_0000,
            bridge: (dc_byte & 0b0100_0000) == 0b0100_0000,
            ipmi_event_generator: (dc_byte & 0b0010_0000) == 0b0010_0000,
            ipmi_event_receiver: (dc_byte & 0b0001_0000) == 0b0001_0000,
            fru_inventory_device: (dc_byte & 0b0000_1000) == 0b0000_1000,
            sel_device: (dc_byte & 0b0000_0100) == 0b0000_0100,
            sdr_repository_device: (dc_byte & 0b0000_0010) == 0b0000_0010,
            sensor_device: (dc_byte & 0b0000_0001) == 0b0000_0001,
        };

        // 3 reserved bytes

        let entity_id = record_data[7];
        let entity_instance = record_data[8];
        let oem_reserved = record_data[9];

        let id_string_type_len = record_data[10];
        let id_string_bytes = &record_data[11..];

        let id_string = TypeLengthRaw::new(id_string_type_len, id_string_bytes).into();

        Some(McDeviceLocatorRecord {
            key,
            acpi_system_power_state_notification_required,
            acpi_device_power_state_notification_required,
            static_controller,
            controller_logs_initialization_errors,
            log_initialization_errors_accessing_controller,
            global_initialization,
            device_capabilities,
            entity_id,
            entity_instance,
            oem_reserved,
            id_string,
        })
    }

    pub fn id_string(&self) -> &SensorId {
        &self.id_string
    }
}
