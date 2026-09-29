#![deny(missing_docs)]
//! IPMI-connection related data.

mod completion_code;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;
use core::num::NonZeroU8;

pub use completion_code::CompletionErrorCode;

mod netfn;
pub use netfn::NetFn;

#[cfg(feature = "alloc")]
mod request;
#[cfg(feature = "alloc")]
pub use request::{Request, RequestTargetAddress};

#[cfg(feature = "alloc")]
mod response;
#[cfg(feature = "alloc")]
pub use response::Response;

/// The address of an IPMI module or sensor.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Address(pub u8);

/// A numbered channel.
///
/// The value of a channel is always less than `0xB`.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ChannelNumber(NonZeroU8);

impl ChannelNumber {
    /// Create a new `ChannelNumber`.
    ///
    /// This function returns `None` if `value > 0xB`
    pub fn new(value: NonZeroU8) -> Option<Self> {
        if value.get() <= 0xB {
            Some(Self(value))
        } else {
            None
        }
    }

    /// Get the value of this `ChannelNumber`.
    ///
    /// It is guaranteed that values returned by
    /// this function are less than or equal to `0xB`
    pub fn value(&self) -> NonZeroU8 {
        self.0
    }
}

/// The channel on which an IPMI module or sensor is present.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Channel {
    /// The primary channel.
    Primary,
    /// A numbered channel.
    Numbered(ChannelNumber),
    /// The system channel.
    System,
    /// The current channel, for some definition of current.
    Current,
}

impl Channel {
    /// Create a new `Channel`.
    ///
    /// This function returns `None` for invalid channel values. `value` is invalid if `value == 0xC || value == 0xD || value > 0xF`.
    pub fn new(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Primary),
            0xE => Some(Self::Current),
            0xF => Some(Self::System),
            v => Some(Self::Numbered(ChannelNumber::new(NonZeroU8::new(v)?)?)),
        }
    }

    /// The number of this channel.
    ///
    /// This value is guaranteed to be less than or
    /// equal to 0xF.
    pub fn value(&self) -> u8 {
        match self {
            Channel::Primary => 0x0,
            Channel::Numbered(v) => v.value().get(),
            Channel::Current => 0xE,
            Channel::System => 0xF,
        }
    }
}

impl core::fmt::Display for Channel {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Channel::Primary => write!(f, "Primary channel"),
            Channel::Numbered(number) => write!(f, "Channel 0x{:01X}", number.value()),
            Channel::Current => write!(f, "Current channel"),
            Channel::System => write!(f, "System channel"),
        }
    }
}

/// The logical unit of an IPMI module/sensor.
#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(missing_docs)]
pub enum LogicalUnit {
    Zero,
    One,
    Two,
    Three,
}

impl LogicalUnit {
    /// Construct a `LogicalUnit` from the two lowest bits of `value`,
    /// ignoring all other bits.
    pub fn from_low_bits(value: u8) -> Self {
        let value = value & 0b11;

        match value {
            0b00 => Self::Zero,
            0b01 => Self::One,
            0b10 => Self::Two,
            0b11 => Self::Three,
            _ => unreachable!("Value bitmasked with 0b11 has value greater than 3"),
        }
    }
}

impl TryFrom<u8> for LogicalUnit {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value <= 0b11 {
            Ok(Self::from_low_bits(value))
        } else {
            Err(())
        }
    }
}

impl LogicalUnit {
    /// Get a raw value describing this logical unit.
    ///
    /// This value is always in the range `0..=3`.
    pub fn value(&self) -> u8 {
        match self {
            LogicalUnit::Zero => 0,
            LogicalUnit::One => 1,
            LogicalUnit::Two => 2,
            LogicalUnit::Three => 3,
        }
    }
}

impl From<LogicalUnit> for u8 {
    fn from(value: LogicalUnit) -> Self {
        value.value()
    }
}

/// A generic error indicating that the message did not contain
/// enough data to constitute a valid response.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NotEnoughData;

/// A trait describing operations that can be performed on an IPMI connection.
#[cfg(feature = "alloc")]
pub trait IpmiConnection {
    /// The type of error the can occur when sending a [`Request`].
    type SendError: core::fmt::Debug;
    /// The type of error that can occur when receiving a [`Response`].
    type RecvError: core::fmt::Debug;
    /// The type of error the can occur when sending a [`Request`] or receiving a [`Response`].
    type Error: core::fmt::Debug + From<Self::SendError> + From<Self::RecvError>;

    /// Send `request` to the remote end of this connection.
    fn send(&mut self, request: &mut Request) -> Result<(), Self::SendError>;

    /// Receive a response from the remote end of this connection.
    fn recv(&mut self) -> Result<Response, Self::RecvError>;

    /// Send `request` to and receive a response from the remote end of this connection.
    fn send_recv(&mut self, request: &mut Request) -> Result<Response, Self::Error>;
}

/// An encoded IPMI request that borrows its command data from caller-provided storage.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EncodedRequest<'a> {
    netfn: NetFn,
    cmd: u8,
    data: &'a [u8],
}

impl EncodedRequest<'_> {
    /// Get the request network function.
    pub const fn netfn(&self) -> NetFn {
        self.netfn
    }

    /// Get the request command byte.
    pub const fn cmd(&self) -> u8 {
        self.cmd
    }

    /// Get the encoded command data.
    pub const fn data(&self) -> &[u8] {
        self.data
    }
}

/// The caller-provided request buffer is too small.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EncodeError {
    /// Number of bytes required by the command.
    pub required: usize,
    /// Number of bytes available in the provided buffer.
    pub available: usize,
}

impl core::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "request buffer needs {} bytes but only {} are available",
            self.required, self.available
        )
    }
}

/// An IPMI command that can encode its request without allocating.
pub trait EncodeIpmiCommand {
    /// Network function for this command.
    const NETFN: NetFn;
    /// Command byte within the network function.
    const CMD: u8;

    /// Number of request-data bytes this command encodes.
    fn request_data_len(&self) -> usize;

    /// Write request data into a buffer of exactly [`Self::request_data_len`] bytes.
    fn write_request_data(&self, data: &mut [u8]);

    /// Encode this request into caller-provided storage.
    fn encode_request<'a>(&self, buffer: &'a mut [u8]) -> Result<EncodedRequest<'a>, EncodeError> {
        let required = self.request_data_len();
        let available = buffer.len();
        let data = buffer.get_mut(..required).ok_or(EncodeError {
            required,
            available,
        })?;
        self.write_request_data(data);
        Ok(EncodedRequest {
            netfn: Self::NETFN,
            cmd: Self::CMD,
            data,
        })
    }
}

/// The wire representation of an IPMI message.
#[cfg(feature = "alloc")]
#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    netfn: u8,
    cmd: u8,
    data: Vec<u8>,
}

#[cfg(feature = "alloc")]
impl Message {
    /// Create a new request message with the provided `netfn`, `cmd` and `data`.
    pub fn new_request(netfn: NetFn, cmd: u8, data: Vec<u8>) -> Self {
        Self {
            netfn: netfn.request_value(),
            cmd,
            data,
        }
    }

    /// Create a new response message with the provided `netfn`, `cmd` and `data`.
    pub fn new_response(netfn: NetFn, cmd: u8, data: Vec<u8>) -> Self {
        Self {
            netfn: netfn.response_value(),
            cmd,
            data,
        }
    }

    /// Create a new message with the provided raw `netfn`, `cmd` and `data`.
    pub fn new_raw(netfn: u8, cmd: u8, data: Vec<u8>) -> Self {
        Self { netfn, cmd, data }
    }

    /// Create an owned request message from an allocation-free command encoder.
    pub fn from_command<C: EncodeIpmiCommand>(command: &C) -> Self {
        let mut data = alloc::vec![0; command.request_data_len()];
        command.write_request_data(&mut data);
        Self::new_request(C::NETFN, C::CMD, data)
    }

    /// Get the netfn of the message.
    pub fn netfn(&self) -> NetFn {
        NetFn::from(self.netfn)
    }

    /// Get the raw netfn value for the message.
    pub fn netfn_raw(&self) -> u8 {
        self.netfn
    }

    /// Get the command value for this message.
    pub fn cmd(&self) -> u8 {
        self.cmd
    }

    /// Get a reference to the data for this message.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Get a mutable reference to the data for this message.
    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }
}

#[cfg(feature = "alloc")]
impl<C: EncodeIpmiCommand> From<C> for Message {
    fn from(command: C) -> Self {
        Self::from_command(&command)
    }
}

/// An IPMI command whose response data can be parsed.
///
/// Request encoding is provided by [`EncodeIpmiCommand`] without requiring allocation.
pub trait IpmiCommand: EncodeIpmiCommand {
    /// The output of this command, i.e. the expected response type.
    type Output;
    /// The type of error that can occur while parsing the response for this
    /// command.
    type Error;

    /// Handle the provided completion code `completion_code` and optionally provide
    /// a special error in case of failure.
    ///
    /// Non-success completion codes for which this function returns `None` should be
    /// handled by the caller of `parse_success_response`.
    ///
    /// The default implementation of this function performs no special handling
    /// and returns `None`.
    #[allow(unused)]
    fn handle_completion_code(
        completion_code: CompletionErrorCode,
        data: &[u8],
    ) -> Option<Self::Error> {
        None
    }

    /// Try to parse the expected response for this command from the
    /// provided `data`, assuming a successful completion code.
    fn parse_success_response(data: &[u8]) -> Result<Self::Output, Self::Error>;

    /// Get the intended target [`Address`] and [`Channel`] for this command.
    fn target(&self) -> Option<(Address, Channel)> {
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::app::GetChannelInfo;

    use super::{Channel, EncodeError, EncodeIpmiCommand, NetFn};

    #[test]
    fn command_encodes_into_caller_buffer() {
        let command = GetChannelInfo::new(Channel::Current);
        let mut buffer = [0_u8; 1];

        let request = command.encode_request(&mut buffer).unwrap();

        assert_eq!(request.netfn(), NetFn::App);
        assert_eq!(request.cmd(), 0x42);
        assert_eq!(request.data(), &[0x0e]);
    }

    #[test]
    fn command_reports_required_buffer_size() {
        let command = GetChannelInfo::new(Channel::Current);

        let error = command.encode_request(&mut []).unwrap_err();

        assert_eq!(
            error,
            EncodeError {
                required: 1,
                available: 0,
            }
        );
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn owned_message_uses_allocation_free_encoder() {
        let message = super::Message::from(GetChannelInfo::new(Channel::Current));

        assert_eq!(message.netfn(), NetFn::App);
        assert_eq!(message.cmd(), 0x42);
        assert_eq!(message.data(), &[0x0e]);
    }
}
