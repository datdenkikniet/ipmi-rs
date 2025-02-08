#![deny(missing_docs)]
//! Implementations & connection-related details.

mod completion_code;
use std::num::NonZeroU8;

pub use completion_code::CompletionCode;

mod impls;

#[cfg(feature = "unix-file")]
pub use impls::File;

pub use impls::rmcp;

mod netfn;
pub use netfn::NetFn;

mod request;
pub use request::{Request, RequestTargetAddress};

mod response;
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

/// A trait describing operations that can be performed on an IPMI connection.
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

    /// Send `request` to and reveive a response from the remote end of this connection.
    fn send_recv(&mut self, request: &mut Request) -> Result<Response, Self::Error>;
}

/// The wire representation of an IPMI messag.e
#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    netfn: u8,
    cmd: u8,
    data: Vec<u8>,
}

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

/// Generic errors that can occur while parsing a [`Response`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParseResponseError<T> {
    /// An error, described by a [`CompletionCode`], occurred.
    Failed(CompletionCode),
    /// There was not enough data to parse the expected [`Response`].
    NotEnoughData,
    /// A different parsing error occurred.
    Parse(T),
}

impl<T> From<T> for ParseResponseError<T> {
    fn from(value: T) -> Self {
        Self::Parse(value)
    }
}

/// An IPMI command that can be turned into a request, and whose response can be parsed
/// from response data.
pub trait IpmiCommand: Into<Message> {
    /// The output of this command, i.e. the expected response type.
    type Output;
    /// The type of error that can occur while parsing the response for this
    /// command.
    type Error;

    /// Try to parse the expected response for this command from the
    /// provided [`CompletionCode`] and `data`.
    // TODO: `parse_response` should only be called if `completion_code` is valid (i.e. only impl `parse_valid_response(&[u8])`)
    fn parse_response(
        completion_code: CompletionCode,
        data: &[u8],
    ) -> Result<Self::Output, ParseResponseError<Self::Error>>;

    /// Convenience function for validating that `cc` is valid, and returning the appropriate
    /// error otherwise.
    fn check_cc_success(cc: CompletionCode) -> Result<(), ParseResponseError<Self::Error>> {
        if cc.is_success() {
            Ok(())
        } else {
            Err(ParseResponseError::Failed(cc))
        }
    }

    /// Get the intended target [`Address`] and [`Channel`] for this commmand.
    fn target(&self) -> Option<(Address, Channel)> {
        None
    }
}
