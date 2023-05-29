use ipmi_rs::{
    connection::{File, Message},
    connection::{IpmiConnection, LogicalUnit, Request},
};
use std::time::Duration;

fn try_parse_message(input: &[u8]) -> Result<Message, String> {
    if input.len() < 2 {
        return Err("Need at least 2 bytes of input".to_string());
    }

    let cmd = input[1];

    let data: Vec<u8> = input[2..].iter().map(|v| *v).collect();

    Ok(Message::new_raw(input[0], cmd, data))
}

fn main() -> Result<(), String> {
    pretty_env_logger::init();

    let mut data = Vec::new();
    for arg in std::env::args().skip(1) {
        let u8_value = u8::from_str_radix(&arg, 16)
            .map_err(|_| format!("Could not parse '{arg}' as hex integer"))?;
        data.push(u8_value);
    }

    let message = try_parse_message(&data)?;

    let mut request: Request = Request::new(message, LogicalUnit::Zero);

    let mut file = File::new("/dev/ipmi0", Duration::from_millis(4000)).unwrap();

    let result = file.send_recv(&mut request).map_err(|e| format!("{e}"))?;

    println!("Response:");
    println!("Completion code: 0x{:02X}", result.cc());
    println!("NetFN: 0x{:02X} ({:?})", result.netfn_raw(), result.netfn());
    println!("Cmd: 0x{:02X}", result.cmd());
    println!("Data: {:02X?}", result.data());
    Ok(())
}
