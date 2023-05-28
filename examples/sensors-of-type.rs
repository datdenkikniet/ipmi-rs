use ipmi_rs::{connection::File, storage::SensorType, Ipmi};
use log::Level;
use std::time::Duration;

fn main() -> Result<(), String> {
    pretty_env_logger::init();

    let args: Vec<_> = std::env::args().skip(1).collect();

    if args.len() != 1 {
        return Err(format!("Expected 1 argument, got {}", args.len()));
    }

    let sensor_type = if let Ok(ty) = SensorType::try_from(args[0].as_str()) {
        ty
    } else {
        return Err(format!("Unknown sensor type '{}'", args[0]));
    };

    let file = File::new("/dev/ipmi0", Duration::from_millis(4000)).unwrap();
    let mut ipmi = Ipmi::new(file);

    let sdrs_of_type = ipmi.sdrs().filter(|s| {
        s.common_data()
            .map(|c| c.ty == sensor_type)
            .unwrap_or(false)
    });

    for sdr in sdrs_of_type {
        ipmi_rs::Logger::log(&Level::Info.into(), &sdr);
    }

    Ok(())
}
