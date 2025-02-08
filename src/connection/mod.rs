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

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Address(pub u8);

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

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Channel {
    Primary,
    Numbered(ChannelNumber),
    System,
    Current,
}

impl Channel {
    /// Create a new `Channel`.
    ///
    /// This function returns `None` if `value == 0xC` || value == 0xD || value > 0xF`
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
    /// equal to 0xF, and will be 0xE if `self` is
    /// [`Channel::Current`].
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

#[derive(Clone, Copy, Debug, PartialEq)]
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

pub trait IpmiConnection {
    type SendError: core::fmt::Debug;
    type RecvError: core::fmt::Debug;
    type Error: core::fmt::Debug + From<Self::SendError> + From<Self::RecvError>;

    fn send(&mut self, request: &mut Request) -> Result<(), Self::SendError>;
    fn recv(&mut self) -> Result<Response, Self::RecvError>;
    fn send_recv(&mut self, request: &mut Request) -> Result<Response, Self::Error>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    netfn: u8,
    cmd: u8,
    data: Vec<u8>,
}

impl Message {
    pub fn new_request(netfn: NetFn, cmd: u8, data: Vec<u8>) -> Self {
        Self {
            netfn: netfn.request_value(),
            cmd,
            data,
        }
    }

    pub fn new_response(netfn: NetFn, cmd: u8, data: Vec<u8>) -> Self {
        Self {
            netfn: netfn.response_value(),
            cmd,
            data,
        }
    }

    pub fn new_raw(netfn: u8, cmd: u8, data: Vec<u8>) -> Self {
        Self { netfn, cmd, data }
    }

    pub fn netfn(&self) -> NetFn {
        NetFn::from(self.netfn)
    }

    pub fn netfn_raw(&self) -> u8 {
        self.netfn
    }

    pub fn cmd(&self) -> u8 {
        self.cmd
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParseResponseError<T> {
    Failed(CompletionCode),
    NotEnoughData,
    Parse(T),
}

impl<T> From<T> for ParseResponseError<T> {
    fn from(value: T) -> Self {
        Self::Parse(value)
    }
}

pub trait IpmiCommand: Into<Message> {
    type Output;
    type Error;

    fn parse_response(
        completion_code: CompletionCode,
        data: &[u8],
    ) -> Result<Self::Output, ParseResponseError<Self::Error>>;

    fn check_cc_success(cc: CompletionCode) -> Result<(), ParseResponseError<Self::Error>> {
        if cc.is_success() {
            Ok(())
        } else {
            Err(ParseResponseError::Failed(cc))
        }
    }

    fn target(&self) -> Option<(Address, Channel)> {
        None
    }
}
