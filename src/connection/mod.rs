mod completion_code;
pub use completion_code::CompletionCode;

mod file;
pub use file::File;

mod netfn;
pub use netfn::{NetFn, NetFns};

mod request;
pub use request::Request;

mod response;
pub use response::{ParsedResponse, Response};

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

    fn send(&mut self, request: &Request) -> Result<(), Self::SendError>;
    fn recv(&mut self) -> Result<Response, Self::RecvError>;
    fn send_recv(&mut self, request: &Request) -> Result<Response, Self::Error>;
}
