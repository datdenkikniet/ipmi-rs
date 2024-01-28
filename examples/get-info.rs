mod common;

use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use ipmi_rs::{
    app::GetDeviceId,
    sensor_event::{GetSensorReading, ThresholdReading},
    storage::sdr::{
        record::RecordContents, GetDeviceSdrInfo, GetSdrAllocInfo, GetSdrRepositoryInfo, SdrCount,
        SdrOperation,
    },
    storage::sel::{GetSelAllocInfo, GetSelEntry, GetSelInfo, RecordId as SelRecordId, SelCommand},
    LogOutput, SensorRecord,
};

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
    ipmi_rs::Logger::log(log_output, &info);

    if info.supported_cmds.contains(&SelCommand::GetAllocInfo) {
        log::info!("Getting SEL Alloc info");
        let alloc_info = ipmi.send_recv(GetSelAllocInfo).unwrap();
        ipmi_rs::Logger::log(log_output, &alloc_info);
    } else {
        log::info!("Getting SEL Alloc info is not supported");
    }

    if info.entries > 0 {
        log::info!("Getting first record");
        let first_record = ipmi
            .send_recv(GetSelEntry::new(None, SelRecordId::LAST))
            .unwrap();

        ipmi_rs::Logger::log(log_output, &first_record);
    }

    let device_id = ipmi.send_recv(GetDeviceId).unwrap();
    ipmi_rs::Logger::log(log_output, &device_id);

    log::info!("Getting Device SDR Info");
    if let Ok(sdr_info) = ipmi.send_recv(GetDeviceSdrInfo::new(SdrCount)) {
        ipmi_rs::Logger::log(log_output, &sdr_info);
    } else {
        log::warn!("Could not get Device SDR info");
    }

    log::info!("Getting SDR repository info");
    let sdr_info = ipmi.send_recv(GetSdrRepositoryInfo).unwrap();
    ipmi_rs::Logger::log(log_output, &sdr_info);

    if sdr_info.supported_ops.contains(&SdrOperation::GetAllocInfo) {
        let sdr_alloc_info = ipmi.send_recv(GetSdrAllocInfo).unwrap();
        ipmi_rs::Logger::log(log_output, &sdr_alloc_info);
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
        if let RecordContents::FullSensor(full) = &sensor.contents {
            let value = ipmi
                .send_recv(GetSensorReading::for_sensor_key(full.key_data()))
                .unwrap();

            let reading: ThresholdReading = (&value).into();

            if let Some(reading) = reading.reading {
                if let Some(display) = full.display_reading(reading) {
                    log::info!("{}: {}", full.id_string(), display);
                    ipmi_rs::Logger::log(debug_log_output, sensor);
                }
            } else {
                log::warn!("No reading for {}", full.id_string());
            }
        } else if let RecordContents::CompactSensor(compact) = &sensor.contents {
            log::info!("Compact sensor {}", compact.id_string(),);
            log::info!("  Sensor type: {:?}", compact.common().ty,);
        } else if let RecordContents::EventOnlySensor(event) = &sensor.contents {
            log::info!("Event-only sensor {}", event.id_string,);
            log::info!("  Sensor type: {:?}", event.ty,);
        } else if let RecordContents::FruDeviceLocator(fru_device_locator) = &sensor.contents {
            log::info!("FRU Device Locator {}", fru_device_locator.id_string());
            log::info!("  Device type: {}", fru_device_locator.device_type);
        } else if let RecordContents::McDeviceLocator(mc_device_locator) = &sensor.contents {
            log::info!("MC Device Locator {}", mc_device_locator.id_string());
        } else if let RecordContents::Unknown { ty, .. } = &sensor.contents {
            log::info!("Unknown record type. Type: 0x{ty:02X}");
        }
    }

    Ok(())
}
