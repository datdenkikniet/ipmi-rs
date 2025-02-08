use crate::connection::{IpmiCommand, Message, NetFn, NotEnoughData};

/// The Get Device ID command.
pub struct GetDeviceId;

impl From<GetDeviceId> for Message {
    fn from(_: GetDeviceId) -> Self {
        Message::new_request(NetFn::App, 0x01, Vec::new())
    }
}

impl IpmiCommand for GetDeviceId {
    type Output = DeviceId;

    type Error = NotEnoughData;

    fn parse_success_response(data: &[u8]) -> Result<Self::Output, Self::Error> {
        DeviceId::from_data(data).ok_or(NotEnoughData)
    }
}

/// All of the fields that are returned when retrieving a
/// device's ID.
#[derive(Clone, Debug, PartialEq)]
pub struct DeviceId {
    /// The raw ID of the device.
    pub device_id: u8,
    /// The revision of the device.
    pub device_revision: u8,
    /// `true` if the device provides device SDRs.
    pub provides_device_sdrs: bool,
    /// `true` if the device is availalbe, `false` if the device
    /// is in device firmware, SDR repository update, or self-initialization state.
    pub device_available: bool,
    /// The major version of the firmware revision of the device.
    pub major_fw_revision: u8,
    /// The minor version of the firmware of the device.
    pub minor_fw_revision: u8,
    /// The major version of the IPMI version supported by the device.
    pub major_version: u8,
    /// The minor version of the IPMI version supported by the device.
    pub minor_version: u8,
    /// `true` if the device is a chassis device per the ICBM specification.
    pub chassis_support: bool,
    /// `true` if the device will response to bridge NetFN commands.
    pub bridge_support: bool,
    /// Whether the device will generate event messages onto the IPMB.
    pub ipmb_event_generator_support: bool,
    /// Whether the device will generate event messages onto the IPMB.
    pub ipmb_event_receiver_support: bool,
    /// Whether if the device supports FRU inventory.
    pub fru_inventory_support: bool,
    /// Whether the device supports the SEL.
    pub sel_device_support: bool,
    /// Whether the device is an SDR repository device.
    pub sdr_repository_support: bool,
    /// Whether the device is a sensor device.
    pub sensor_device_support: bool,
    /// The ID of the manufacturer.
    pub manufacturer_id: u32,
    /// The ID of the product.
    pub product_id: u16,
    /// Optional auxiliary firmware revision information.
    pub aux_revision: Option<[u8; 4]>,
}

impl DeviceId {
    /// Parse a `DeviceID` from IPMI response data.
    pub fn from_data(data: &[u8]) -> Option<Self> {
        if data.len() < 11 {
            return None;
        }

        let aux_revision = if data.len() < 15 {
            None
        } else {
            Some([data[11], data[12], data[13], data[14]])
        };

        let fw_min = {
            let min_nib_low = data[3] & 0xF;
            let min_nib_high = (data[3] >> 4) & 0xF;

            min_nib_low + min_nib_high * 10
        };

        let me = Self {
            device_id: data[0],
            device_revision: data[1] & 0xF,
            provides_device_sdrs: (data[1] & 0x80) == 0x80,
            device_available: (data[2] & 0x80) != 0x80,
            major_fw_revision: (data[2] & 0x7F),
            minor_fw_revision: fw_min,
            major_version: data[4] & 0xF,
            minor_version: (data[4] >> 4) & 0xF,
            chassis_support: (data[5] & 0x80) == 0x80,
            bridge_support: (data[5] & 0x40) == 0x40,
            ipmb_event_generator_support: (data[5] & 0x20) == 0x20,
            ipmb_event_receiver_support: (data[5] & 0x10) == 0x10,
            fru_inventory_support: (data[5] & 0x08) == 0x08,
            sel_device_support: (data[5] & 0x04) == 0x04,
            sdr_repository_support: (data[5] & 0x02) == 0x02,
            sensor_device_support: (data[5] & 0x01) == 0x01,
            manufacturer_id: u32::from_le_bytes([data[6], data[7], data[8], 0]),
            product_id: u16::from_le_bytes([data[9], data[10]]),
            aux_revision,
        };

        Some(me)
    }
}
