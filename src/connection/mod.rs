mod completion_code;
pub use completion_code::CompletionCode;

mod file;
pub use file::File;

mod netfn;
pub use netfn::NetFn;

mod request;
pub use request::Request;

mod response;
pub use response::Response;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LogicalUnit(u8);

impl LogicalUnit {
    pub const ZERO: Self = Self(0);
    pub const ONE: Self = Self(1);
    pub const TWO: Self = Self(2);
    pub const THREE: Self = Self(3);
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
    netfn: NetFn,
    cmd: u8,
    data: Vec<u8>,
}

impl Message {
    pub fn new(netfn: NetFn, cmd: u8, data: Vec<u8>) -> Self {
        Self { netfn, cmd, data }
    }

    pub fn netfn(&self) -> NetFn {
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
}
