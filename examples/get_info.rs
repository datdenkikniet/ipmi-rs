use std::time::Duration;

use ipmi_rs::{
    connection::File, connection::NetFns, AppCommand, AppNetFn, Ipmi, LogOutput, Loggable,
    StorageCommand,
};

fn main() {
    pretty_env_logger::init();

    let file = File::new("/dev/ipmi0", Duration::from_millis(2000)).unwrap();
    let mut ipmi = Ipmi::new(file);
    let log_output = log::Level::Info.into();

    let device_id = ipmi
        .send_recv(AppNetFn::request(AppCommand::GetDeviceId).into(), &[])
        .unwrap();

    device_id.log(log_output);

    let sel_info = ipmi.get_sel_info().unwrap();

    sel_info.log(log_output);

    if sel_info
        .supported_cmds
        .contains(&StorageCommand::GetSelAllocInfo)
    {
        let sel_alloc_info = ipmi.get_sel_alloc_info().unwrap();

        sel_alloc_info.log(log_output);
    } else {
        ipmi_rs::log!(log_output, "No SEL Alloc information available");
    }
}
