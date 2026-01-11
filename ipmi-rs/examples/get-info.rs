mod common;

use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use ipmi_rs::{
    app::GetDeviceId,
    connection::CompletionErrorCode,
    sensor_event::{GetSensorReading, ThresholdReading},
    storage::{
        sdr::{
            record::{IdentifiableSensor, InstancedSensor, RecordContents},
            GetDeviceSdrInfo, GetSdrRepositoryInfo, SdrCount, SdrGetAllocInfo, SdrOperation,
        },
        sel::{GetSelEntry, GetSelInfo, RecordId as SelRecordId, SelCommand, SelGetAllocInfo},
    },
    IpmiError,
};
use ipmi_rs_log::{LogOutput, Logger};

fn main() -> std::io::Result<()> {
    pretty_env_logger::formatted_builder()
        .parse_filters(&std::env::var("RUST_LOG").unwrap_or("info".to_string()))
        .init();

    let opts = common::CliOpts::parse();

    let mut ipmi = opts.get_connection()?;

    let log_output = &LogOutput::LogTarget(log::Level::Info, "get_info".into());
    let debug_log_output = &LogOutput::LogTarget(log::Level::Debug, "get_info".into());

    log::info!("Getting SEL info");
    let info = ipmi.send_recv(GetSelInfo).unwrap();
    Logger::log(log_output, &info);

    if info.supported_cmds.contains(&SelCommand::GetAllocInfo) {
        log::info!("Getting SEL Alloc info");
        let alloc_info = ipmi.send_recv(SelGetAllocInfo).unwrap();
        Logger::log(log_output, &alloc_info);
    } else {
        log::info!("Getting SEL Alloc info is not supported");
    }

    if info.entries > 0 {
        log::info!("Getting first record");
        let first_record = ipmi
            .send_recv(GetSelEntry::new(None, SelRecordId::LAST))
            .unwrap();

        Logger::log(log_output, &first_record);
    }

    let device_id = ipmi.send_recv(GetDeviceId).unwrap();
    Logger::log(log_output, &device_id);

    log::info!("Getting Device SDR Info");
    if let Ok(sdr_info) = ipmi.send_recv(GetDeviceSdrInfo::new(SdrCount)) {
        Logger::log(log_output, &sdr_info);
    } else {
        log::warn!("Could not get Device SDR info");
    }

    log::info!("Getting SDR repository info");
    let sdr_info = ipmi.send_recv(GetSdrRepositoryInfo).unwrap();
    Logger::log(log_output, &sdr_info);

    if sdr_info.supported_ops.contains(&SdrOperation::GetAllocInfo) {
        let sdr_alloc_info = ipmi.send_recv(SdrGetAllocInfo).unwrap();
        Logger::log(log_output, &sdr_alloc_info);
    };

    let template = "[{bar:.green/white}] {prefix} ({pos}/{len})";

    let progress_bar = ProgressBar::new(sdr_info.record_count as u64)
        .with_style(
            ProgressStyle::with_template(&template)
                .unwrap()
                .progress_chars("#>-"),
        )
        .with_prefix("Loading SDR Records");

    let sensors = ipmi
        .sdrs()
        .map(|v| {
            progress_bar.inc(1);
            v
        })
        .collect::<Vec<_>>();

    progress_bar.finish();

    for sensor in &sensors {
        match &sensor.contents {
            RecordContents::FullSensor(full) => {
                log_id("Full Sensor Record", full);
                let result = ipmi.send_recv(GetSensorReading::for_sensor_key(full.key_data()));
                let value = match result {
                    Ok(value) => value,
                    Err(IpmiError::Failed {
                        completion_code: CompletionErrorCode::RequestedDatapointNotPresent,
                        ..
                    }) => {
                        log::warn!("  Sensor for {} not present", full.id_string());
                        continue;
                    }
                    Err(e) => {
                        log::error!(
                            "Failed to get sensor reading for {}: {e:?}",
                            full.id_string()
                        );
                        continue;
                    }
                };

                let reading: ThresholdReading = (&value).into();

                if let Some(reading) = reading.reading {
                    if let Some(display) = full.display_reading(reading) {
                        log::info!("  {}: {}", full.id_string(), display);
                        Logger::log(debug_log_output, sensor);
                    }
                } else {
                    log::warn!("  No reading for {}", full.id_string());
                }
            }
            RecordContents::CompactSensor(compact) => log_sensor("Compact Sensor", compact),
            RecordContents::EventOnlySensor(event) => log_sensor("Event-only Sensor", event),
            RecordContents::GenericDeviceLocator(generic) => {
                log_id("Generic Device Locator", generic);
                log_device_type(generic.device_type, generic.device_type_modifier);
            }
            RecordContents::FruDeviceLocator(fru) => {
                log_id("FRU Device Locator", fru);
                log_device_type(fru.device_type, fru.device_type_modifier);
            }
            RecordContents::McDeviceLocator(mc) => log_id("MC Device Locator", mc),
            RecordContents::Unknown { ty, .. } => {
                log::info!("Unknown record type. Type: 0x{ty:02X}");
            }
        }
    }

    Ok(())
}

fn log_id<T: IdentifiableSensor>(ty: &str, sensor: &T) {
    log::info!("{ty} {}", sensor.id_string());
}

fn log_sensor<T: InstancedSensor>(ty: &str, sensor: &T) {
    log_id(ty, sensor);
    log::info!("  Sensor type: {:?}", sensor.ty());
}

fn log_device_type(device_type: u8, device_type_modifier: u8) {
    let name = device_type_name(device_type);
    match name {
        Some(name) => log::info!(
            "  Device type: {} (0x{device_type:02X}, {name})",
            device_type
        ),
        None => log::info!("  Device type: {} (0x{device_type:02X})", device_type),
    }

    let modifier_name = device_type_modifier_name(device_type, device_type_modifier);
    match modifier_name {
        Some(name) => log::info!(
            "  Device type modifier: 0x{device_type_modifier:02X} ({name})"
        ),
        None => log::info!("  Device type modifier: 0x{device_type_modifier:02X}"),
    }
}

fn device_type_name(device_type: u8) -> Option<&'static str> {
    // IPMI 2.0 Specification, Table 43-12 "IPMB/I2C Device Type Codes".
    match device_type {
        0x00 => Some("Reserved"),
        0x01 => Some("Reserved"),
        0x02 => Some("DS1624 temperature sensor / EEPROM or equivalent"),
        0x03 => Some("DS1621 temperature sensor or equivalent"),
        0x04 => Some("LM75 temperature sensor or equivalent"),
        0x05 => Some("Heceta ASIC or similar"),
        0x06 => Some("Reserved"),
        0x07 => Some("Reserved"),
        0x08 => Some("EEPROM, 24C01 or equivalent"),
        0x09 => Some("EEPROM, 24C02 or equivalent"),
        0x0A => Some("EEPROM, 24C04 or equivalent"),
        0x0B => Some("EEPROM, 24C08 or equivalent"),
        0x0C => Some("EEPROM, 24C16 or equivalent"),
        0x0D => Some("EEPROM, 24C17 or equivalent"),
        0x0E => Some("EEPROM, 24C32 or equivalent"),
        0x0F => Some("EEPROM, 24C64 or equivalent"),
        0x10 => Some(
            "FRU Inventory Device behind management controller (Read/Write FRU at LUN != 00b)",
        ),
        0x11 => Some("Reserved"),
        0x12 => Some("Reserved"),
        0x13 => Some("Reserved"),
        0x14 => Some("PCF8570 256 byte RAM or equivalent"),
        0x15 => Some("PCF8573 clock/calendar or equivalent"),
        0x16 => Some("PCF8574A I/O port or equivalent"),
        0x17 => Some("PCF8583 clock/calendar or equivalent"),
        0x18 => Some("PCF8593 clock/calendar or equivalent"),
        0x19 => Some("Clock calendar, type not specified"),
        0x1A => Some("PCF8591 A/D, D/A Converter or equivalent"),
        0x1B => Some("I/O port, specific device not specified"),
        0x1C => Some("A/D Converter, specific device not specified"),
        0x1D => Some("D/A Converter, specific device not specified"),
        0x1E => Some("A/D, D/A Converter, specific device not specified"),
        0x1F => Some("LCD controller/Driver, specific device not specified"),
        0x20 => Some("Core Logic (chip set) device, specific device not specified"),
        0x21 => Some("LMC6874 Intelligent Battery controller, or equivalent"),
        0x22 => Some("Intelligent Battery controller, specific device not specified"),
        0x23 => Some("Combo Management ASIC, specific device not specified"),
        0x24 => Some("Maxim 1617 temperature sensor"),
        0xBF => Some("Other/unspecified device"),
        0xC0..=0xFF => Some("OEM specified device"),
        _ => Some("Reserved"),
    }
}

fn device_type_modifier_name(device_type: u8, modifier: u8) -> Option<&'static str> {
    // IPMI 2.0 Specification, Table 43-12 "IPMB/I2C Device Type Codes".
    match device_type {
        0x05 => match modifier {
            0x00 => Some("Heceta 1 (LM78)"),
            0x01 => Some("Heceta 2 (LM79)"),
            0x02 => Some("LM80"),
            0x03 => Some("Heceta 3 (LM81/ADM9240/DS1780)"),
            0x04 => Some("Heceta 4"),
            0x05 => Some("Heceta 5"),
            _ => Some("Reserved"),
        },
        0x08..=0x0F => eeprom_modifier_name(modifier),
        0x10 => fru_behind_mc_modifier_name(modifier),
        0xBF => modifier_unspecified_or_none(modifier),
        0xC0..=0xFF => Some("OEM specific"),
        _ => modifier_unspecified_or_none(modifier),
    }
}

fn modifier_unspecified_or_none(modifier: u8) -> Option<&'static str> {
    if modifier == 0x00 {
        Some("Unspecified")
    } else {
        None
    }
}

fn eeprom_modifier_name(modifier: u8) -> Option<&'static str> {
    match modifier {
        0x00 => Some("Unspecified"),
        0x01 => Some("DIMM Memory ID"),
        0x02 => Some("IPMI FRU Inventory"),
        0x03 => Some("System Processor Cartridge FRU/PIROM"),
        _ => Some("Reserved"),
    }
}

fn fru_behind_mc_modifier_name(modifier: u8) -> Option<&'static str> {
    match modifier {
        0x00 => Some("IPMI FRU Inventory"),
        0x01 => Some("DIMM Memory ID"),
        0x02 => Some("IPMI FRU Inventory"),
        0x03 => Some("System Processor Cartridge FRU/PIROM"),
        0xFF => Some("Unspecified"),
        _ => Some("Reserved"),
    }
}
