//! IPMI-rs: a pure-rust IPMI library.
//!
//! This library provides command serialization and deserialization (in the [`app`], [`storage`] and [`sensor_event`] modules),
//! and different ways of connecting to an IPMI device (in the [`connection`] module).

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
    CompletionErrorCode, IpmiCommand, LogicalUnit, NetFn, Request, RequestTargetAddress,
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
    ) -> Result<CMD::Output, IpmiError<CON::Error, CMD::Error>>
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

        let map_error = |completion_code, error| IpmiError::Command {
            error,
            netfn: response.netfn(),
            cmd: response.cmd(),
            completion_code,
            data: response.data().to_vec(),
        };

        if let Ok(completion_code) = CompletionErrorCode::try_from(response.cc()) {
            let error = CMD::handle_completion_code(completion_code, response.data())
                .map(|e| IpmiError::Command {
                    error: e,
                    netfn: response.netfn(),
                    cmd: response.cmd(),
                    completion_code: Some(completion_code),
                    data: response.data().to_vec(),
                })
                .unwrap_or_else(|| IpmiError::Failed {
                    netfn: response.netfn(),
                    cmd: response.cmd(),
                    completion_code,
                    data: response.data().to_vec(),
                });

            return Err(error);
        }

        CMD::parse_success_response(response.data()).map_err(|err| map_error(None, err))
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
                Err(IpmiError::Command {
                    error: (e, Some(next_record_id)),
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
