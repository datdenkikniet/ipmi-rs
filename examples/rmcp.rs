use ipmi_rs::{
    app::{Channel, GetChannelAuthenticationCapabilities, PrivilegeLevel},
    connection::Rmcp,
    Ipmi,
};

fn main() -> std::io::Result<()> {
    let rmcp = Rmcp::new("172.19.202.33:623")?;
    let mut ipmi = Ipmi::new(rmcp);

    let output = ipmi.send_recv(GetChannelAuthenticationCapabilities::new(
        Channel::Current,
        PrivilegeLevel::Administrator,
    ));

    println!("{:#?}", output);

    Ok(())
}
