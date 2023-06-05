use std::{io::ErrorKind, time::Duration};

use clap::Parser;
use ipmi_rs::{
    connection::{
        rmcp::{Active, Rmcp},
        File, IpmiCommand,
    },
    storage::sdr,
    Ipmi, IpmiCommandError, SdrIter,
};

#[allow(unused)]
fn main() {}

pub enum IpmiConnectionEnum {
    Rmcp(Ipmi<Rmcp<Active>>),
    File(Ipmi<File>),
}

enum SdrIterInner<'a> {
    Rmcp(SdrIter<'a, Rmcp<Active>>),
    File(SdrIter<'a, File>),
}

impl Iterator for SdrIterInner<'_> {
    type Item = sdr::Record;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            SdrIterInner::Rmcp(rmcp) => rmcp.next(),
            SdrIterInner::File(file) => file.next(),
        }
    }
}

impl IpmiConnectionEnum {
    pub fn send_recv<CMD>(
        &mut self,
        request: CMD,
    ) -> Result<CMD::Output, IpmiCommandError<std::io::Error, CMD::Error>>
    where
        CMD: IpmiCommand,
    {
        match self {
            IpmiConnectionEnum::Rmcp(rmcp) => rmcp.send_recv(request),
            IpmiConnectionEnum::File(file) => file.send_recv(request),
        }
    }

    pub fn sdrs(&mut self) -> impl Iterator<Item = sdr::Record> + '_ {
        match self {
            IpmiConnectionEnum::Rmcp(rmcp) => SdrIterInner::Rmcp(rmcp.sdrs()),
            IpmiConnectionEnum::File(file) => SdrIterInner::File(file.sdrs()),
        }
    }
}

#[derive(Parser)]
pub struct CliOpts {
    /// The connection URI to use
    #[clap(default_value = "file:///dev/ipmi0", long, short)]
    connection_uri: String,
    /// How many milliseconds to wait before timing out while waiting for a response
    #[clap(default_value = "2000", long)]
    timeout_ms: u64,
}

fn error<T>(val: T) -> std::io::Error
where
    T: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    std::io::Error::new(ErrorKind::Other, val)
}

impl CliOpts {
    pub fn get_connection(&self) -> std::io::Result<IpmiConnectionEnum> {
        let timeout = Duration::from_millis(self.timeout_ms);

        if self.connection_uri.starts_with("file://") {
            let (_, path) = self.connection_uri.split_once("file://").unwrap();

            log::debug!("Opening file {path}");

            let file = File::new(path, timeout)?;
            let ipmi = Ipmi::new(file);
            Ok(IpmiConnectionEnum::File(ipmi))
        } else if self.connection_uri.starts_with("rmcp://") {
            let (_, data) = self.connection_uri.split_once("rmcp://").unwrap();

            let err =
                || error("Invalid connection URI. Format: `rmcp://[username]:[password]@[address]");

            let (username, rest) = data.split_once(':').ok_or(err())?;

            let (password, address) = rest.split_once('@').ok_or(err())?;

            log::debug!("Opening connection to {address}");

            let rmcp = Rmcp::new(address, timeout)?;
            let activated = rmcp
                .activate(Some(username), password.as_bytes())
                .map_err(|e| error(format!("RMCP activation error: {:?}", e)))?;

            let ipmi = Ipmi::new(activated);
            Ok(IpmiConnectionEnum::Rmcp(ipmi))
        } else {
            Err(error(format!(
                "Invalid connection URI {}",
                self.connection_uri
            )))
        }
    }
}
