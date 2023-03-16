pub mod app;
pub use app::{Command as AppCommand, NetFn as AppNetFn};

pub mod connection;
use connection::{LogicalUnit, NetFn, NetFns, ParsedResponse, Request, Response};

pub mod storage;
pub use storage::{Command as StorageCommand, NetFn as StorageNetFn};
use storage::{GetSelEntry, SelAllocInfo, SelEntry, SelInfo, SelRecordId};

#[macro_use]
mod fmt;
pub use fmt::{LogOutput, Loggable};

pub struct Ipmi<T> {
    inner: T,
    counter: i64,
}

impl<T> Ipmi<T> {
    pub fn new(inner: T) -> Self {
        Self { inner, counter: 0 }
    }
}

impl<T> From<T> for Ipmi<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum IpmiError<T> {
    NetFnIsResponse(NetFn),
    IncorrectResponseSeq {
        seq_sent: i64,
        seq_recvd: i64,
    },
    UnexpectedResponse {
        netfn_sent: NetFn,
        netfn_recvd: NetFn,
    },
    ResponseParseFailed(<ParsedResponse as TryFrom<Response>>::Error),
    Connection(T),
}

impl<T> From<T> for IpmiError<T> {
    fn from(value: T) -> Self {
        Self::Connection(value)
    }
}

impl<T> Ipmi<T>
where
    T: connection::IpmiConnection,
{
    pub fn send_recv(&mut self, netfn: NetFn) -> Result<ParsedResponse, IpmiError<T::Error>> {
        if netfn.is_response() {
            return Err(IpmiError::NetFnIsResponse(netfn));
        }

        let seq = self.counter;
        self.counter += 1;

        let request = Request::new(netfn.clone(), LogicalUnit::ONE, seq);

        let response = self.inner.send_recv(&request)?;

        if response.seq() != seq {
            return Err(IpmiError::IncorrectResponseSeq {
                seq_sent: seq,
                seq_recvd: response.seq(),
            });
        }

        if !response.netfn().is_response_for(&netfn) {
            return Err(IpmiError::UnexpectedResponse {
                netfn_sent: netfn.clone(),
                netfn_recvd: response.netfn().clone(),
            });
        }

        response
            .try_into()
            .map_err(|e| IpmiError::ResponseParseFailed(e))
    }

    pub fn get_sel_entry(
        &mut self,
        record_id: SelRecordId,
    ) -> Result<(SelEntry, SelRecordId), IpmiError<T::Error>> {
        let result = self.send_recv(NetFn::Storage(StorageNetFn::request(
            StorageCommand::GetSelEntry(Some(GetSelEntry {
                reservation_id: None,
                record_id,
                offset: 0,
                bytes_to_read: None,
            })),
        )))?;

        match result {
            ParsedResponse::SelEntry { next_entry, entry } => Ok((entry, next_entry)),
            _ => unreachable!(),
        }
    }
}

macro_rules! get_parsed {
    ($($name:ident => $command:expr => $out:ty => $out_variant:ident),*) => {
        impl<T: connection::IpmiConnection> Ipmi<T> {
            $(
                pub fn $name(&mut self) -> Result<$out, IpmiError<T::Error>> {
                    let response = self.send_recv($command.into())?;

                    match response {
                        ParsedResponse::$out_variant(value) => Ok(value),
                        _ => unreachable!(),
                    }
                }
            )*
        }
    };
}

get_parsed!(
    get_sel_info => StorageNetFn::request(StorageCommand::GetSelInfo) => SelInfo => SelInfo,
    get_sel_alloc_info => StorageNetFn::request(StorageCommand::GetSelAllocInfo) => SelAllocInfo => SelAllocInfo
);
