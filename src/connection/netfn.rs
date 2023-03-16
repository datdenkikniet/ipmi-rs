pub use crate::storage::{Command as StorageCommand, NetFn as StorageNetFn};

pub use crate::app::{Command as AppCommand, NetFn as AppNetFn};

pub trait NetFns {
    type Command;

    fn request(cmd: Self::Command) -> Self;
    fn response(cmd: Self::Command) -> Self;
    fn is_response(&self) -> bool;
    fn cmd(&self) -> Self::Command;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NetFn {
    App(AppNetFn),
    Storage(StorageNetFn),
    Unknown(u8, u8),
}

macro_rules! direct_from {
    ($($from:ty => $to:ident),*) => {
        $(
            impl From<$from> for NetFn {
                fn from(value: $from) -> Self {
                    Self::$to(value)
                }
            }
        )*
    };
}

direct_from!(
    AppNetFn => App,
    StorageNetFn => Storage
);

impl NetFn {
    pub fn from_parts(netfn: u8, cmd: u8) -> Self {
        match netfn {
            0x0A => StorageNetFn::request(cmd.into()).into(),
            0x0B => StorageNetFn::response(cmd.into()).into(),
            0x06 => AppNetFn::request(cmd.into()).into(),
            0x07 => AppNetFn::response(cmd.into()).into(),
            netfn => Self::Unknown(netfn, cmd),
        }
    }

    pub fn parts(&self) -> (u8, u8) {
        match self {
            NetFn::Storage(str_netfn) => {
                let netfn = if !str_netfn.is_response() { 0x0A } else { 0x0B };
                let cmd = str_netfn.cmd().into();
                (netfn, cmd)
            }
            NetFn::App(str_netfn) => {
                let netfn = if !str_netfn.is_response() { 0x06 } else { 0x07 };
                let cmd = str_netfn.cmd().into();
                (netfn, cmd)
            }
            NetFn::Unknown(netfn, cmd) => (*netfn, *cmd),
        }
    }

    pub fn is_response(&self) -> bool {
        match self {
            NetFn::Storage(netfn) => netfn.is_response(),
            NetFn::App(netfn) => netfn.is_response(),
            NetFn::Unknown(netfn, _) => netfn % 2 == 1,
        }
    }

    pub fn is_response_for(&self, other: &Self) -> bool {
        let (net_fn, cmd) = self.parts();
        let (other_net_fn, other_cmd) = other.parts();

        (other_net_fn + 1 == net_fn) && cmd == other_cmd
    }
}
