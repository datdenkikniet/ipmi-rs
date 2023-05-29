use ipmi_rs::connection::Rmcp;

fn main() -> std::io::Result<()> {
    Rmcp::new("172.19.202.33:623").map(|_| ())
}
