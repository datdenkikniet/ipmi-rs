mod completion_code;
pub use completion_code::CompletionCode;

mod impls;

#[cfg(feature = "unix-file")]
pub use impls::File;

pub use impls::rmcp;

mod netfn;
pub use netfn::NetFn;

mod request;
pub use request::Request;

mod response;
pub use response::Response;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LogicalUnit {
    Zero,
    One,
    Two,
    Three,
}

impl TryFrom<u8> for LogicalUnit {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let val = match value {
            0 => Self::Zero,
            1 => Self::One,
            2 => Self::Two,
            3 => Self::Three,
            _ => return Err(()),
        };
        Ok(val)
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

    fn address_and_channel(&self) -> Option<(u8, u8)> {
        None
    }
}
