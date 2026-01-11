//! Generic Device Locator Record (SDR Type 10h)
//!
//! Reference: IPMI 2.0 Specification, Table 43-6 "SDR Type 10h - Generic Device Locator Record"

use crate::connection::LogicalUnit;
use crate::storage::sdr::record::{SensorId, TypeLengthRaw};

use super::{IdentifiableSensor, ParseError};
use std::num::NonZeroU8;

/// Record key for Generic Device Locator Record (SDR Type 10h).
///
/// Reference: IPMI 2.0 Specification, Table 43-6, bytes 6-8
/// (record data offsets 0-2).
#[derive(Debug, Clone)]
pub struct GenericDeviceRecordKey {
    /// 7-bit I2C Slave Address of device on the channel.
    pub device_access_address: u8,
    /// 7-bit I2C Slave Address on the device's bus.
    pub device_slave_address: u8,
    /// Channel number for the management controller used to access the device.
    pub channel_number: u8,
    /// Access LUN for Master Write-Read command.
    pub access_lun: LogicalUnit,
    /// Private bus ID if bus is private, None if device directly on IPMB.
    pub private_bus_id: Option<NonZeroU8>,
}

/// Generic Device Locator Record (SDR Type 10h).
///
/// This record is used to store the location and type information for devices
/// on the IPMB or management controller private busses that are neither IPMI
/// FRU devices nor IPMI management controllers.
///
/// Reference: IPMI 2.0 Specification, Section 43.7 and Table 43-6
#[derive(Debug, Clone)]
pub struct GenericDeviceLocator {
    /// Record key data.
    pub record_key: GenericDeviceRecordKey,
    /// Address span (number of addresses device occupies - 1).
    pub address_span: u8,
    /// Device Type code per IPMI Device Type Codes table.
    ///
    /// Reference: IPMI 2.0 Specification, Table 43-12 "Device Type Codes"
    pub device_type: u8,
    /// Device Type Modifier.
    ///
    /// Reference: IPMI 2.0 Specification, Table 43-6
    pub device_type_modifier: u8,
    /// Entity ID for the device.
    ///
    /// Reference: IPMI 2.0 Specification, Table 43-13 "Entity ID Codes"
    pub entity_id: u8,
    /// Entity Instance.
    ///
    /// Note: The IPMI spec only labels this as "Entity Instance" (Table 43-6)
    /// without the sensor SDR bit layout, so we keep it as a raw `u8`.
    pub entity_instance: u8,
    /// OEM reserved field.
    pub oem_reserved: u8,
    /// Device ID string.
    pub id_string: SensorId,
}

impl IdentifiableSensor for GenericDeviceLocator {
    fn id_string(&self) -> &SensorId {
        &self.id_string
    }

    fn entity_id(&self) -> u8 {
        self.entity_id
    }
}

impl GenericDeviceLocator {
    /// Parse a Generic Device Locator Record from raw SDR record data.
    ///
    /// The record data layout is defined in IPMI 2.0 Specification, Table 43-6.
    /// Offsets below are relative to the record data payload (table bytes 6+).
    ///
    /// | Offset | Field                              |
    /// |--------|-----------------------------------|
    /// | 0      | Device Access Address [7:1], [0] reserved |
    /// | 1      | Device Slave Address, channel ms-bit in [0] |
    /// | 2      | [7:5] Channel Number (ls-3 bits), [4:3] Access LUN, [2:0] Private Bus ID |
    /// | 3      | [7:3] reserved, [2:0] Address Span |
    /// | 4      | Reserved                          |
    /// | 5      | Device Type (Table 43-12)         |
    /// | 6      | Device Type Modifier (Table 43-12)|
    /// | 7      | Entity ID (Table 43-13)           |
    /// | 8      | Entity Instance                   |
    /// | 9      | OEM                               |
    /// | 10     | Device ID String Type/Length      |
    /// | 11+    | Device ID String bytes            |
    pub fn parse(record_data: &[u8]) -> Result<Self, ParseError> {
        if record_data.len() < 11 {
            return Err(ParseError::NotEnoughData);
        }

        // Byte 0: Device Access Address
        // [7:1] = 7-bit I2C slave address of device on channel
        // [0] = reserved
        //
        // Reference: IPMI 2.0 Spec, Table 43-6
        let device_access_address = record_data[0] >> 1;

        // Byte 1: Device Slave Address / Device ID
        //
        // Reference: IPMI 2.0 Spec, Table 43-6
        let device_slave_address = record_data[1] >> 1;

        // Byte 2: Access LUN / Bus ID
        // [7:5] = Channel Number (ls-3 bits)
        // [4:3] = LUN for Master Write-Read command
        // [2:0] = Private bus ID (0 if device directly on IPMB)
        //
        // Reference: IPMI 2.0 Spec, Table 43-6
        let access_lun = LogicalUnit::from_low_bits(record_data[2] >> 3);
        let private_bus_id = NonZeroU8::new(record_data[2] & 0b111);
        let channel_number = ((record_data[1] & 0b1) << 3) | (record_data[2] >> 5);

        // Byte 3: Address Span
        // [7:3] = reserved
        // [2:0] = Address span (number of addresses device uses - 1)
        //
        // Reference: IPMI 2.0 Spec, Table 43-6
        let address_span = record_data[3] & 0b111;

        // Byte 4: Reserved
        //
        // Reference: IPMI 2.0 Spec, Table 43-6

        // Byte 5: Device Type
        // Device type code per Table 43-12 "Device Type Codes"
        //
        // Reference: IPMI 2.0 Spec, Table 43-6 and Table 43-12
        let device_type = record_data[5];

        // Byte 6: Device Type Modifier
        //
        // Reference: IPMI 2.0 Spec, Table 43-6
        let device_type_modifier = record_data[6];

        // Byte 7: Entity ID
        //
        // Reference: IPMI 2.0 Spec, Table 43-6 and Table 43-13
        let entity_id = record_data[7];

        // Byte 8: Entity Instance
        //
        // Reference: IPMI 2.0 Spec, Table 43-6
        let entity_instance = record_data[8];

        // Byte 9: OEM
        // Reserved for OEM use
        //
        // Reference: IPMI 2.0 Spec, Table 43-6
        let oem_reserved = record_data[9];

        // Byte 10: Device ID String Type/Length
        // [7:6] = Type code (00=Unicode, 01=BCD+, 10=6-bit ASCII, 11=8-bit ASCII+Latin1)
        // [4:0] = Length of string in bytes
        // Byte 11+: Device ID String bytes
        //
        // Reference: IPMI 2.0 Spec, Table 43-6 and Section 43.15 "Type/Length Byte Format"
        let id_string_type_len = record_data[10];
        let id_string_bytes = &record_data[11..];
        let id_string = TypeLengthRaw::new(id_string_type_len, id_string_bytes).try_into()?;

        let record_key = GenericDeviceRecordKey {
            device_access_address,
            device_slave_address,
            channel_number,
            access_lun,
            private_bus_id,
        };

        Ok(Self {
            record_key,
            address_span,
            device_type,
            device_type_modifier,
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
