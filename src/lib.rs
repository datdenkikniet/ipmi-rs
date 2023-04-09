pub mod app;

pub mod connection;

pub mod storage;
pub use storage::sdr::record::SensorRecord;

pub mod sensor_event;

#[macro_use]
mod fmt;
pub use fmt::{LogOutput, Loggable, Logger};

use connection::{IpmiCommand, LogicalUnit, NetFn, ParseResponseError, Request};
use storage::{sdr::record::Record as SdrRecord, GetDeviceSdr, SdrRecordId};

pub struct Ipmi<CON> {
    inner: CON,
    counter: i64,
}

impl<CON> Ipmi<CON> {
    pub fn new(inner: CON) -> Self {
        Self { inner, counter: 0 }
    }
}

impl<CON> From<CON> for Ipmi<CON> {
    fn from(value: CON) -> Self {
        Self::new(value)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum IpmiError<CON, P> {
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
    Connection(CON),
}

impl<T, P> From<T> for IpmiError<T, P> {
    fn from(value: T) -> Self {
        Self::Connection(value)
    }
}

pub type IpmiCommandError<T, E> = IpmiError<T, ParseResponseError<E>>;

impl<CON> Ipmi<CON>
where
    CON: connection::IpmiConnection,
{
    pub fn sdrs(&mut self) -> impl Iterator<Item = SdrRecord> + '_ {
        SdrIter {
            ipmi: self,
            next_id: Some(SdrRecordId::FIRST),
        }
    }

    pub fn send_recv<CMD>(
        &mut self,
        request: CMD,
    ) -> Result<CMD::Output, IpmiCommandError<CON::Error, CMD::Error>>
    where
        CMD: IpmiCommand,
    {
        let seq = self.counter;
        self.counter += 1;

        let message = request.into();
        let (message_netfn, message_cmd) = (message.netfn(), message.cmd());
        let mut request = Request::new(message, LogicalUnit::One, seq);

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

        CMD::parse_response(response.cc().into(), response.data()).map_err(|error| {
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

pub struct SdrIter<'ipmi, CON> {
    ipmi: &'ipmi mut Ipmi<CON>,
    next_id: Option<SdrRecordId>,
}

impl<T> Iterator for SdrIter<'_, T>
where
    T: connection::IpmiConnection,
{
    type Item = SdrRecord;

    fn next(&mut self) -> Option<Self::Item> {
        let next_id = self.next_id?;
        let next_record = self
            .ipmi
            .send_recv(GetDeviceSdr::new(None, next_id))
            .map_err(|e| {
                log::error!("Error occured while iterating SDR records: {e:?}");
            })
            .ok()?;

        if !next_record.next_entry.is_last() {
            self.next_id = Some(next_record.next_entry);
        } else {
            self.next_id.take();
        }

        Some(next_record.record)
    }
}
