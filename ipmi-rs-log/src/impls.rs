use std::{num::NonZeroU16, ops::Deref};

use crate::{log_vec, LogItem, Loggable};

use ipmi_rs_core::{
    app::DeviceId,
    storage::{
        sdr::{
            record::{
                IdentifiableSensor, InstancedSensor, RecordHeader, SensorKey, SensorOwner, Value,
            },
            DeviceSdrInfo, NumberOfSdrs, NumberOfSensors, Record, SdrAllocInfo, SdrRepositoryInfo,
        },
        sel::{
            Entry, EventDirection, EventMessageRevision, SelAllocInfo, SelCommand, SelEntryInfo,
            SelInfo,
        },
        AllocInfo,
    },
};

impl Loggable for SelInfo {
    fn as_log(&self) -> Vec<crate::LogItem> {
        let (ver_maj, ver_min) = (self.version_maj, self.version_min);

        let supported_cmds: Vec<_> = self
            .supported_cmds
            .iter()
            .map(|cmd| match cmd {
                SelCommand::GetAllocInfo => "Get Alloc Info",
                SelCommand::Clear => "Clear",
                SelCommand::PartialAddEntry => "Partial Add",
                SelCommand::Reserve => "Reserve",
            })
            .collect();

        log_vec![
            (0, "SEL information"),
            (1, "Version", format!("{}.{}", ver_maj, ver_min)),
            (1, "Entries", self.entries),
            (1, "Bytes free", self.bytes_free),
            (1, "Last addition", self.last_add_time),
            (1, "Last erase", self.last_del_time),
            (1, "Overflowed", self.overflow),
            (1, "Supported cmds", format!("{:?}", supported_cmds)),
        ]
    }
}

impl Loggable for SelEntryInfo {
    fn as_log(&self) -> Vec<crate::LogItem> {
        let mut log_output = self.entry.as_log();

        let value = format!("0x{:04X}", self.next_entry.value());
        log_output.push((1, "Next entry", value).into());
        log_output
    }
}

impl Loggable for SelAllocInfo {
    fn as_log(&self) -> Vec<crate::LogItem> {
        let mut alloc_log_output = Loggable::as_log(self.deref());
        alloc_log_output.insert(0, (0, "SEL Allocation Information").into());
        alloc_log_output
    }
}

impl Loggable for Entry {
    fn as_log(&self) -> Vec<LogItem> {
        match self {
            Entry::System {
                record_id,
                timestamp,
                generator_id,
                event_message_format,
                sensor_type,
                sensor_number,
                event_direction,
                event_type,
                event_data,
            } => {
                let format = match event_message_format {
                    EventMessageRevision::V2_0 => "2.0".into(),
                    EventMessageRevision::V1_0 => "1.0".into(),
                    EventMessageRevision::Unknown(v) => format!("Unknown (0x{:02X})", v),
                };

                let event_dir = match event_direction {
                    EventDirection::Assert => "Asserted",
                    EventDirection::Deassert => "Deasserted",
                };

                log_vec![
                    (0, "SEL entry"),
                    (1, "Record type", "System (0x02)"),
                    (1, "Record ID", format!("0x{:04X}", record_id.value())),
                    (1, "Time", timestamp),
                    (1, "Generator", format!("{:?}", generator_id)),
                    (1, "Format revision", format),
                    (1, "Sensor type", format!("0x{sensor_type:02X}")),
                    (1, "Sensor number", format!("0x{sensor_number:02X}")),
                    (1, "Assertion state", event_dir),
                    (1, "Event type", format!("0x{event_type:02X}")),
                    (1, "Data", format!("{event_data:02X?}")),
                ]
            }
            Entry::OemTimestamped {
                record_id,
                ty,
                timestamp,
                manufacturer_id,
                data,
            } => {
                log_vec![
                    (0, "SEL entry"),
                    (1, "Record type", format!("Timestamped OEM (0x{ty:08X})")),
                    (1, "Record ID", format!("0x{:04X}", record_id.value())),
                    (1, "Type", format!("{ty:02X}")),
                    (1, "Timestamp", timestamp),
                    (1, "Manufacturer ID", format!("{manufacturer_id:02X?}")),
                    (1, "Data", format!("{data:02X?}")),
                ]
            }
            Entry::OemNotTimestamped {
                record_id,
                ty,
                data,
            } => {
                log_vec![
                    (0, "SEL entry"),
                    (1, "Record type", format!("Not timestamp OEM (0x{ty:08X}")),
                    (1, "Record ID", format!("0x{:04X}", record_id.value())),
                    (1, "Type", format!("0x{ty:02X}")),
                    (1, "Data", format!("{data:02X?}"))
                ]
            }
        }
    }
}

impl Loggable for AllocInfo {
    fn as_log(&self) -> Vec<LogItem> {
        let unspecified_if_zero = |v: Option<NonZeroU16>| {
            if let Some(v) = v {
                format!("{}", v.get())
            } else {
                "Unspecified".into()
            }
        };

        let num_alloc_units = unspecified_if_zero(self.num_alloc_units);
        let alloc_unit_size = unspecified_if_zero(self.alloc_unit_size);

        log_vec!(
            (1, "# of units", num_alloc_units),
            (1, "Unit size", alloc_unit_size),
            (1, "# free units", self.num_free_units),
            (1, "Largest free block", self.largest_free_blk),
            (1, "Max record size", self.max_record_size),
        )
    }
}

impl Loggable for SdrRepositoryInfo {
    fn as_log(&self) -> Vec<LogItem> {
        let Self {
            version_major,
            version_minor,
            record_count,
            free_space,
            most_recent_addition,
            most_recent_erase,
            overflow,
            supported_ops,
        } = self;

        let (v_maj, v_min) = (version_major, version_minor);

        log_vec![
            (0, "SDR Repository Information"),
            (1, "Version", format!("{v_maj}.{v_min}")),
            (1, "Record count", record_count),
            (1, "Free space", free_space),
            (1, "Most recent add", most_recent_addition),
            (1, "Most recent erase", most_recent_erase),
            (1, "SDR Overflow", overflow),
            (1, "Supported ops", format!("{supported_ops:?}"))
        ]
    }
}

impl Loggable for DeviceSdrInfo<NumberOfSdrs> {
    fn as_log(&self) -> Vec<crate::LogItem> {
        let log = log_vec![
            (0, "Device SDR information"),
            (1, "Number of SDRs", self.operation_value.0)
        ];
        partial_log_device_sdr(self, log)
    }
}

impl Loggable for DeviceSdrInfo<NumberOfSensors> {
    fn as_log(&self) -> Vec<crate::LogItem> {
        let log = log_vec![
            (0, "Device SDR information"),
            (1, "Number of sensors", self.operation_value.0)
        ];

        partial_log_device_sdr(self, log)
    }
}

fn partial_log_device_sdr<T>(
    device: &DeviceSdrInfo<T>,
    mut log: Vec<crate::LogItem>,
) -> Vec<crate::LogItem> {
    let mut luns_with_sensors = Vec::new();
    if device.lun_0_has_sensors {
        luns_with_sensors.push(0);
    }
    if device.lun_1_has_sensors {
        luns_with_sensors.push(1);
    }
    if device.lun_2_has_sensors {
        luns_with_sensors.push(2);
    }
    if device.lun_3_has_sensors {
        luns_with_sensors.push(3);
    }

    log.push((1, "LUNs with sensors", format!("{:?}", luns_with_sensors)).into());

    if let Some(epoch) = device.sensor_population_epoch {
        log.push((1, "Sensor pop. epoch", format!("0x{epoch:04X}")).into());
    }

    log
}

impl Loggable for SdrAllocInfo {
    fn as_log(&self) -> Vec<crate::LogItem> {
        let mut log = Loggable::as_log(self.deref());
        log.insert(0, (0, "SDR Repository Allocation Information").into());
        log
    }
}

impl Loggable for SensorKey {
    fn as_log(&self) -> Vec<LogItem> {
        let sensor_owner = match self.owner_id {
            SensorOwner::I2C(addr) => format!("I2C @ 0x{:02X}", addr),
            SensorOwner::System(addr) => format!("System @ 0x{:02X}", addr),
        };

        log_vec![
            (0, "Sensor owner", sensor_owner),
            (0, "Owner channel", self.owner_channel),
            (0, "Owner LUN", self.owner_lun.value()),
            (0, "Sensor number", self.sensor_number.get())
        ]
    }
}

impl Loggable for DeviceId {
    fn as_log(&self) -> Vec<crate::LogItem> {
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

impl Loggable for Record {
    fn as_log(&self) -> Vec<crate::LogItem> {
        let full = self.full_sensor();
        let compact = self.compact_sensor();
        let event_only = self.event_only();

        let mut log = Vec::new();

        if full.is_some() {
            log.push((0, "SDR Record (Full)").into());
        } else if compact.is_some() {
            log.push((0, "SDR Record (Compact)").into());
        } else if event_only.is_some() {
            log.push((0, "SDR Record (Event-only)").into())
        } else {
            log.push((0, "Cannot log unknown sensor type").into());
            return log;
        }

        let RecordHeader {
            id,
            sdr_version_major: sdr_v_maj,
            sdr_version_minor: sdr_v_min,
        } = &self.header;

        log.push((1, "Record ID", format!("0x{:04X}", id.value())).into());
        log.push((1, "SDR Version", format!("{sdr_v_maj}.{sdr_v_min}")).into());

        if let Some(common) = self.common_data() {
            log.push((1, "Sensor Type", format!("{:?}", common.ty)).into());
        }

        if let Some(full) = full {
            let mut key_log = full.key_data().as_log();
            key_log.iter_mut().for_each(|v| v.level += 1);
            log.append(&mut key_log);

            let display = |v: Value| v.display(true);

            let nominal_reading = full
                .nominal_value()
                .map(display)
                .unwrap_or("Unknown".into());

            let max_reading = full.max_reading().map(display).unwrap_or("Unknown".into());
            let min_reading = full.min_reading().map(display).unwrap_or("Unknown".into());

            log.push((1, "Sensor ID", full.id_string()).into());
            log.push((1, "Entity ID", full.entity_id()).into());
            log.push((1, "Nominal reading", nominal_reading).into());
            log.push((1, "Max reading", max_reading).into());
            log.push((1, "Min reading", min_reading).into());
        } else if let Some(compact) = compact {
            let mut key_log = compact.key_data().as_log();
            key_log.iter_mut().for_each(|v| v.level += 1);
            log.append(&mut key_log);
            log.push((1, "Sensor ID", compact.id_string()).into());
        } else if let Some(event_only) = event_only {
            let mut key_log = event_only.key_data().as_log();
            key_log.iter_mut().for_each(|v| v.level += 1);
            log.append(&mut key_log);
            log.push((1, "Sensor ID", event_only.id_string()).into());
        }

        log
    }
}
