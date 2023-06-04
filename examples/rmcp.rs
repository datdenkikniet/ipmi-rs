use ipmi_rs::{
    app::auth::{Channel, GetChannelAuthenticationCapabilities, PrivilegeLevel},
    connection::Rmcp,
    Ipmi,
};

fn main() -> std::io::Result<()> {
    pretty_env_logger::init();

    let rmcp = Rmcp::new("172.19.202.33:623")?;

    let activated = rmcp.activate(Some("root"), b"").unwrap();

    // let mut ipmi = Ipmi::new(rmcp);

    // let output = ipmi.send_recv(GetChannelAuthenticationCapabilities::new(
    //     Channel::Current,
    //     PrivilegeLevel::Administrator,
    // ));

    // println!("{:#?}", output);

    Ok(())
}
