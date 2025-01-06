pub mod app;

pub mod connection;

mod error;
pub use error::IpmiError;

pub mod storage;
pub use storage::sdr::record::WithSensorRecordCommon;

pub mod sensor_event;

#[macro_use]
mod fmt;
#[cfg(test)]
mod tests;

pub use fmt::{LogOutput, Loggable, Logger};

use connection::{
    IpmiCommand, LogicalUnit, NetFn, ParseResponseError, Request, RequestTargetAddress,
};
use storage::sdr::{self, record::Record as SdrRecord};

pub struct Ipmi<CON> {
    inner: CON,
}

impl<CON> Ipmi<CON> {
    pub fn release(self) -> CON {
        self.inner
    }
}

impl<CON> From<CON> for Ipmi<CON>
where
    CON: connection::IpmiConnection,
{
    fn from(value: CON) -> Self {
        Self::new(value)
    }
}

pub type IpmiCommandError<T, E> = IpmiError<T, ParseResponseError<E>>;

impl<CON> Ipmi<CON>
where
    CON: connection::IpmiConnection,
{
    pub fn inner_mut(&mut self) -> &mut CON {
        &mut self.inner
    }

    pub fn new(inner: CON) -> Self {
        Self { inner }
    }

    pub fn sdrs(&mut self) -> SdrIter<CON> {
        SdrIter {
            ipmi: self,
            next_id: Some(sdr::RecordId::FIRST),
        }
    }

    pub fn send_recv<CMD>(
        &mut self,
        request: CMD,
    ) -> Result<CMD::Output, IpmiCommandError<CON::Error, CMD::Error>>
    where
        CMD: IpmiCommand,
    {
        let target_address = match request.target() {
            Some((a, c)) => RequestTargetAddress::BmcOrIpmb(a, c, LogicalUnit::Zero),
            None => RequestTargetAddress::Bmc(LogicalUnit::Zero),
        };

        let message = request.into();
        let (message_netfn, message_cmd) = (message.netfn(), message.cmd());
        let mut request = Request::new(message, target_address);

        let response = self.inner.send_recv(&mut request)?;

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
    next_id: Option<sdr::RecordId>,
}

impl<T> Iterator for SdrIter<'_, T>
where
    T: connection::IpmiConnection,
{
    type Item = SdrRecord;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(current_id) = self.next_id.take() {
            if current_id.is_last() {
                return None;
            }

            let next_record = self
                .ipmi
                .send_recv(sdr::GetDeviceSdr::new(None, current_id));

            match next_record {
                Ok(record) => {
                    let next_record_id = record.next_entry;

                    if next_record_id == current_id {
                        log::error!("Got duplicate SDR record IDs! Stopping iteration.");
                        return None;
                    }

                    self.next_id = Some(next_record_id);
                    return Some(record.record);
                }
                Err(IpmiError::ParsingFailed {
                    error: ParseResponseError::Parse((e, next_record_id)),
                    ..
                }) => {
                    log::warn!(
                        "Recoverable error while parsing SDR record 0x{:04X}: {e:?}. Skipping to next.",
                        current_id.value()
                    );
                    self.next_id = Some(next_record_id);
                    continue; // skip the current one
                }
                Err(e) => {
                    log::error!(
                        "Unrecoverable error while parsing SDR record 0x{:04X}: {e:?}",
                        current_id.value()
                    );
                    return None;
                }
            }
        }
        None
    }
}
