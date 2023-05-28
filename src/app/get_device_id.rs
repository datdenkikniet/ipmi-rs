use crate::{
    connection::{IpmiCommand, Message, NetFn, ParseResponseError},
    log_vec, Loggable,
};

pub struct GetDeviceId;

impl From<GetDeviceId> for Message {
    fn from(_: GetDeviceId) -> Self {
        Message::new_request(NetFn::App, 0x01, Vec::new())
    }
}

impl IpmiCommand for GetDeviceId {
    type Output = DeviceId;

    type Error = ();

    fn parse_response(
        completion_code: crate::connection::CompletionCode,
        data: &[u8],
    ) -> Result<Self::Output, ParseResponseError<Self::Error>> {
        Self::check_cc_success(completion_code)?;
        DeviceId::from_data(data).ok_or(ParseResponseError::NotEnoughData)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeviceId {
    pub device_id: u8,
    pub device_revision: u8,
    pub provides_device_sdrs: bool,
    pub device_available: bool,
    pub major_fw_revision: u8,
    pub minor_fw_revision: u8,
    pub major_version: u8,
    pub minor_version: u8,
    pub chassis_support: bool,
    pub bridge_support: bool,
    pub ipmb_event_generator_support: bool,
    pub ipmb_event_receiver_support: bool,
    pub fru_inventory_support: bool,
    pub sel_device_support: bool,
    pub sdr_repository_support: bool,
    pub sensor_device_support: bool,
    pub manufacturer_id: u32,
    pub product_id: u16,
    pub aux_revision: Option<[u8; 4]>,
}

impl DeviceId {
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

impl Loggable for DeviceId {
    fn into_log(&self) -> Vec<crate::fmt::LogItem> {
        let (dev_id, dev_rev) = (self.device_id, self.device_revision);
        let (fw_maj, fw_min) = (self.major_fw_revision, self.minor_fw_revision);
        let (v_maj, v_min) = (self.major_version, self.minor_version);
        let manf_id = self.manufacturer_id;

        let (ipmb_event_gen, ipmb_event_recv) = (
            self.ipmb_event_generator_support,
            self.ipmb_event_receiver_support,
        );

        let fru_inv = self.fru_inventory_support;
        let sdr_rep = self.sdr_repository_support;
        let sensor_dev = self.sensor_device_support;
        let sdrs = self.provides_device_sdrs;

        let mut log = log_vec![
            (0, "Device ID information"),
            (1, "Device ID", format!("0x{dev_id:02X}")),
            (1, "Device revision", format!("0x{dev_rev:02X}")),
            (1, "Manufacturer ID", format!("0x{manf_id:02X}")),
            (1, "Product ID", format!("0x{:02X}", self.product_id)),
            (1, "IPMI Version", format!("{v_maj}.{v_min}")),
            (1, "FW revision", format!("{fw_maj}.{fw_min}")),
            // Aux revision
            (1, "Device available", self.device_available),
            (1, "Provides device SDRs", sdrs),
            (1, "Chassis support", self.chassis_support),
            (1, "Bridge support", self.bridge_support),
            (1, "IPMB Event gen sup", ipmb_event_gen),
            (1, "IPMB Event recv sup", ipmb_event_recv),
            (1, "FRU Inventory sup", fru_inv),
            (1, "SEL Device support", self.sel_device_support),
            (1, "SDR Repository sup", sdr_rep),
            (1, "Sensor Device sup", sensor_dev)
        ];

        if let Some(aux_rev) = &self.aux_revision {
            let element = (1, "Auxiliary revision", format!("{aux_rev:02X?}")).into();
            log.insert(7, element);
        }

        log
    }
}
