#![allow(unused)]

use std::{io::ErrorKind, time::Duration};

use clap::{Args, Parser};
use ipmi_rs::{
    connection::{
        rmcp::{
            Rmcp, RmcpIpmiError, RmcpIpmiReceiveError, RmcpIpmiSendError, V1_5WriteError,
            V2_0WriteError,
        },
        File, IpmiCommand,
    },
    storage::sdr,
    Ipmi, IpmiError, SdrIter,
};

#[allow(unused)]
fn main() {}

pub enum IpmiConnectionEnum {
    Rmcp(Ipmi<Rmcp>),
    File(Ipmi<File>),
}

enum SdrIterInner<'a> {
    Rmcp(SdrIter<'a, Rmcp>),
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
    ) -> Result<CMD::Output, IpmiError<std::io::Error, CMD::Error>>
    where
        CMD: IpmiCommand,
    {
        match self {
            IpmiConnectionEnum::Rmcp(rmcp) => match rmcp.send_recv(request) {
                Ok(v) => Ok(v),
                Err(e) => {
                    let mapped = e.map(|e| match e {
                        RmcpIpmiError::Receive(RmcpIpmiReceiveError::Io(io))
                        | RmcpIpmiError::Send(RmcpIpmiSendError::V1_5(V1_5WriteError::Io(io)))
                        | RmcpIpmiError::Send(RmcpIpmiSendError::V2_0(V2_0WriteError::Io(io))) => {
                            io
                        }
                        e => {
                            log::error!("RMCP command failed: {e:?}");
                            std::io::Error::new(ErrorKind::Other, format!("{e:?}"))
                        }
                    });

                    Err(mapped)
                }
            },
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
    #[clap(flatten)]
    pub common: CommonOpts,
}

impl CliOpts {
    pub fn get_connection(&self) -> std::io::Result<IpmiConnectionEnum> {
        self.common.get_connection()
    }
}

#[derive(Args)]
pub struct CommonOpts {
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

impl CommonOpts {
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

            let mut rmcp = Rmcp::new(address, timeout)?;
            rmcp.activate(true, Some(username), Some(password.as_bytes()))
                .map_err(|e| error(format!("RMCP activation error: {:?}", e)))?;

            let ipmi = Ipmi::new(rmcp);
            Ok(IpmiConnectionEnum::Rmcp(ipmi))
        } else {
            Err(error(format!(
                "Invalid connection URI {}",
                self.connection_uri
            )))
        }
    }
}
