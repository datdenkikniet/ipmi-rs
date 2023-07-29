use clap::Parser;
use ipmi_rs::storage::sdr::SensorType;
use log::Level;

mod common;

#[derive(Parser)]
pub struct Command {
    #[clap(flatten)]
    common: common::CommonOpts,

    #[clap(required = true)]
    types: Vec<String>,
}

fn main() -> std::io::Result<()> {
    pretty_env_logger::init();

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

    let sdrs_of_type = ipmi.sdrs().filter(|s| {
        s.common_data()
            .map(|c| sensor_types.contains(&c.ty))
            .unwrap_or(false)
    });

    for sdr in sdrs_of_type {
        ipmi_rs::Logger::log(&Level::Info.into(), &sdr);
    }

    Ok(())
}
