use std::time::Duration;

use ipmi_rs::{
    app::GetDeviceId,
    connection::File,
    storage::{GetSelAllocInfo, GetSelEntry, GetSelInfo, SelCommand, SelRecordId},
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
        .send_recv(GetSelEntry::new(None, SelRecordId::FIRST, 0))
        .unwrap();

    first_record.log(log_output);

    let device_id = ipmi.send_recv(GetDeviceId).unwrap();
    device_id.log(log_output);
}
