use std::time::Duration;

use indicatif::{ProgressBar, ProgressStyle};
use ipmi_rs::{
    app::GetDeviceId,
    connection::File,
    sensor_event::{GetSensorReading, ThresholdReading},
    storage::{
        record::RecordContents, GetSdrAllocInfo, GetSdrRepositoryInfo, GetSelAllocInfo,
        GetSelEntry, GetSelInfo, SdrOperation, SelCommand, SelRecordId,
    },
    Ipmi, LogOutput, Loggable, SensorRecord,
};

fn main() {
    pretty_env_logger::init();

    let file = File::new("/dev/ipmi0", Duration::from_millis(4000)).unwrap();
    let mut ipmi = Ipmi::new(file);
    let log_output = &LogOutput::LogTarget(log::Level::Info, "get_info".into());
    let debug_log_output = &LogOutput::LogTarget(log::Level::Debug, "get_info".into());

    log::info!("Getting SEL info");
    let info = ipmi.send_recv(GetSelInfo).unwrap();
    info.log(log_output);

    if info.supported_cmds.contains(&SelCommand::GetAllocInfo) {
        log::info!("Getting SEL Alloc info");
        let alloc_info = ipmi.send_recv(GetSelAllocInfo).unwrap();
        alloc_info.log(log_output);
    } else {
        log::info!("Getting SEL Alloc info is not supported");
    }

    if info.entries > 0 {
        log::info!("Getting first record");
        let first_record = ipmi
            .send_recv(GetSelEntry::new(None, SelRecordId::FIRST))
            .unwrap();

        first_record.log(log_output);
    }

    let device_id = ipmi.send_recv(GetDeviceId).unwrap();
    device_id.log(log_output);

    let sdr_info = ipmi.send_recv(GetSdrRepositoryInfo).unwrap();
    sdr_info.log(log_output);

    if sdr_info.supported_ops.contains(&SdrOperation::GetAllocInfo) {
        let sdr_alloc_info = ipmi.send_recv(GetSdrAllocInfo).unwrap();
        sdr_alloc_info.log(log_output);
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
        .enumerate()
        .map(|(idx, v)| {
            progress_bar.set_position(idx as u64 + 1);
            v
        })
        .collect::<Vec<_>>();

    progress_bar.finish();

    for sensor in &sensors {
        if let RecordContents::FullSensor(full) = &sensor.contents {
            let value = ipmi
                .send_recv(GetSensorReading::for_sensor(full.sensor_number()))
                .unwrap();

            let reading: ThresholdReading = (&value).into();

            if let Some(display) = full.display_reading(reading.reading.unwrap()) {
                log::info!("{}: {}", full.id_string(), display);
                sensor.log(debug_log_output);
            }
        }
    }
}
