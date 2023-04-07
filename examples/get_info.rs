use std::time::Duration;

use ipmi_rs::{
    app::GetDeviceId,
    connection::File,
    sensor_event::GetSensorReading,
    storage::{
        record::{Record, RecordContents},
        GetSdrAllocInfo, GetSdrRepositoryInfo, GetSelAllocInfo, GetSelEntry, GetSelInfo,
        SdrOperation, SelCommand, SelRecordId,
    },
    Ipmi, Loggable,
};

fn main() {
    pretty_env_logger::init();

    let file = File::new("/dev/ipmi0", Duration::from_millis(2000)).unwrap();
    let mut ipmi = Ipmi::new(file);
    let log_output = log::Level::Info.into();

    let info = ipmi.send_recv(GetSelInfo).unwrap();
    info.log(log_output);

    if info.supported_cmds.contains(&SelCommand::GetAllocInfo) {
        let alloc_info = ipmi.send_recv(GetSelAllocInfo).unwrap();
        alloc_info.log(log_output);
    }

    let first_record = ipmi
        .send_recv(GetSelEntry::new(None, SelRecordId::FIRST))
        .unwrap();

    first_record.log(log_output);

    let device_id = ipmi.send_recv(GetDeviceId).unwrap();
    device_id.log(log_output);

    let sdr_info = ipmi.send_recv(GetSdrRepositoryInfo).unwrap();
    sdr_info.log(log_output);

    if sdr_info.supported_ops.contains(&SdrOperation::GetAllocInfo) {
        let sdr_alloc_info = ipmi.send_recv(GetSdrAllocInfo).unwrap();
        sdr_alloc_info.log(log_output);
    }

    let sensors = ipmi.sdrs().collect::<Vec<_>>();
    let sensor_0 = &sensors[0];

    let sensor_0_num = sensor_0.sensor_number().unwrap();

    let sensor_reading = ipmi
        .send_recv(GetSensorReading::for_sensor(sensor_0_num))
        .unwrap();

    match sensor_0 {
        Record {
            header: _,
            contents: RecordContents::FullSensor(full),
        } => {
            log::info!(
                "{}: {}",
                full.id_string,
                full.display_reading(sensor_reading.reading).unwrap()
            )
        }
        _ => {}
    }
}
