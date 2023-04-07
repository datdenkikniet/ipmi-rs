pub mod app;

pub mod connection;

pub mod storage;

pub mod sensor_event;

#[macro_use]
mod fmt;
pub use fmt::{LogOutput, Loggable};

use connection::{IpmiCommand, LogicalUnit, NetFn, ParseResponseError, Request};
use storage::{sdr::record::Record as SdrRecord, GetDeviceSdr, SdrRecordId};

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
pub enum IpmiError<T, P> {
    NetFnIsResponse(NetFn),
    IncorrectResponseSeq {
        seq_sent: i64,
        seq_recvd: i64,
    },
    UnexpectedResponse {
        netfn_sent: NetFn,
        netfn_recvd: NetFn,
        cmd_sent: u8,
        cmd_recvd: u8,
    },
    ParsingFailed {
        error: P,
        netfn: NetFn,
        cmd: u8,
        completion_code: u8,
        data: Vec<u8>,
    },
    Connection(T),
}

impl<T, P> From<T> for IpmiError<T, P> {
    fn from(value: T) -> Self {
        Self::Connection(value)
    }
}

impl<T> Ipmi<T>
where
    T: connection::IpmiConnection,
{
    pub fn sdrs<'a>(&'a mut self) -> impl Iterator<Item = SdrRecord> + 'a {
        SdrIter {
            ipmi: self,
            next_id: Some(SdrRecordId::FIRST),
        }
    }

    pub fn send_recv<C>(
        &mut self,
        request: C,
    ) -> Result<C::Output, IpmiError<T::Error, ParseResponseError<C::Error>>>
    where
        C: IpmiCommand,
    {
        let seq = self.counter;
        self.counter += 1;

        let message = request.into();
        let (message_netfn, message_cmd) = (message.netfn(), message.cmd());
        let mut request = Request::new(message, LogicalUnit::ONE, seq);

        let response = self.inner.send_recv(&mut request)?;

        if response.seq() != seq {
            return Err(IpmiError::IncorrectResponseSeq {
                seq_sent: seq,
                seq_recvd: response.seq(),
            });
        }

        if response.netfn() != message_netfn || response.cmd() != message_cmd {
            return Err(IpmiError::UnexpectedResponse {
                netfn_sent: message_netfn,
                netfn_recvd: response.netfn(),
                cmd_sent: message_cmd,
                cmd_recvd: response.cmd(),
            });
        }

        C::parse_response(response.cc().into(), response.data()).map_err(|error| {
            IpmiError::ParsingFailed {
                error,
                netfn: response.netfn(),
                completion_code: response.cc(),
                cmd: response.cmd(),
                data: response.data().to_vec(),
            }
        })
    }
}

pub struct SdrIter<'ipmi, T> {
    ipmi: &'ipmi mut Ipmi<T>,
    next_id: Option<SdrRecordId>,
}

impl<T> Iterator for SdrIter<'_, T>
where
    T: connection::IpmiConnection,
{
    type Item = SdrRecord;

    fn next(&mut self) -> Option<Self::Item> {
        let next_id = self.next_id?;
        let next_record = self.ipmi.send_recv(GetDeviceSdr::new(None, next_id)).ok()?;

        if !next_record.next_entry.is_last() {
            self.next_id = Some(next_record.next_entry);
        } else {
            self.next_id.take();
        }

        Some(next_record.record)
    }
}
