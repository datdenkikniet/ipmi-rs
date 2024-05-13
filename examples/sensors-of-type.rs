use clap::Parser;
use ipmi_rs::{
    sensor_event::{GetSensorReading, ThresholdReading},
    storage::sdr::{
        record::{IdentifiableSensor, InstancedSensor, RecordContents},
        Record, SensorType,
    },
};
use log::Level;

mod common;

#[derive(Parser)]
pub struct Command {
    #[clap(flatten)]
    common: common::CommonOpts,

    #[clap(required = true)]
    types: Vec<String>,

    #[clap(long, short)]
    reading: bool,
}

fn main() -> std::io::Result<()> {
    pretty_env_logger::formatted_builder()
        .parse_filters(&std::env::var("RUST_LOG").unwrap_or("info".to_string()))
        .init();

    let args = Command::parse();

    let mut sensor_types = Vec::new();

    for ty in args.types {
        if let Ok(ty) = SensorType::try_from(ty.as_str()) {
            sensor_types.push(ty);
        } else {
            let err = std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Unknown sensor type '{}'", ty),
            );
            return Err(err);
        };
    }

    let mut ipmi = args.common.get_connection()?;

    let sdrs_of_type: Vec<Record> = ipmi
        .sdrs()
        .filter(|s| {
            s.common_data()
                .map(|c| sensor_types.contains(&c.ty))
                .unwrap_or(false)
        })
        .collect();

    for sdr in sdrs_of_type {
        ipmi_rs::Logger::log(&Level::Info.into(), &sdr);

        if let RecordContents::FullSensor(full) = sdr.contents {
            if args.reading {
                let value = ipmi
                    .send_recv(GetSensorReading::for_sensor_key(full.key_data()))
                    .unwrap();

                let reading: ThresholdReading = (&value).into();

                if let Some(reading) = reading.reading {
                    if let Some(display) = full.display_reading(reading) {
                        log::info!("Current reading for {}: {}", full.id_string(), display);
                    }
                } else {
                    log::warn!("No reading for {}", full.id_string());
                }
            }
        } else {
            log::warn!(
                "Don't know how to read sensor value of non-full-record {}",
                sdr.id()
                    .map(|v| format!("{v}"))
                    .unwrap_or("Unknown".to_string())
            )
        }
    }

    Ok(())
}
