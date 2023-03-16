pub use crate::storage::{Command as StorageCommand, NetFn as StorageNetFn};

pub use crate::app::{Command as AppCommand, NetFn as AppNetFn};

pub trait NetFns {
    type Command;

    fn request(cmd: Self::Command) -> Self;
    fn response(cmd: Self::Command) -> Self;
    fn is_response(&self) -> bool;
    fn cmd(&self) -> Self::Command;
    fn data(&self) -> Vec<u8>;
}

#[derive(Debug, Clone, PartialEq)]
pub enum NetFn {
    App(AppNetFn),
    Storage(StorageNetFn),
    Unknown(u8, u8, Vec<u8>),
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
    pub fn from_parts(netfn: u8, cmd: u8, data: &[u8]) -> Self {
        match netfn {
            0x0A => StorageNetFn::request(StorageCommand::from_parts(cmd, data)).into(),
            0x0B => StorageNetFn::response(StorageCommand::from_parts(cmd, data)).into(),
            0x06 => AppNetFn::request(AppCommand::from_parts(cmd)).into(),
            0x07 => AppNetFn::response(AppCommand::from_parts(cmd)).into(),
            netfn => Self::Unknown(netfn, cmd, data.iter().map(Clone::clone).collect()),
        }
    }

    fn cmd_id(&self) -> u8 {
        match self {
            NetFn::App(netfn) => netfn.cmd().cmd_id(),
            NetFn::Storage(netfn) => netfn.cmd().cmd_id(),
            NetFn::Unknown(_, cmd_id, _) => *cmd_id,
        }
    }

    pub fn netfn_id(&self) -> u8 {
        match self {
            NetFn::App(netfn) => {
                if netfn.is_response() {
                    0x07
                } else {
                    0x06
                }
            }
            NetFn::Storage(netfn) => {
                if netfn.is_response() {
                    0x0B
                } else {
                    0x0A
                }
            }
            NetFn::Unknown(netfn, _, _) => *netfn,
        }
    }

    pub fn parts(&self) -> (u8, u8, Vec<u8>) {
        match self {
            NetFn::Storage(str_netfn) => {
                let netfn = if !str_netfn.is_response() { 0x0A } else { 0x0B };
                let (cmd, data) = str_netfn.cmd().parts();
                (netfn, cmd, data)
            }
            NetFn::App(app_netfn) => {
                let netfn = if !app_netfn.is_response() { 0x06 } else { 0x07 };
                let cmd = app_netfn.cmd().cmd_id();
                (netfn, cmd, Vec::new())
            }
            NetFn::Unknown(netfn, cmd, data) => (*netfn, *cmd, data.clone()),
        }
    }

    pub fn is_response(&self) -> bool {
        match self {
            NetFn::Storage(netfn) => netfn.is_response(),
            NetFn::App(netfn) => netfn.is_response(),
            NetFn::Unknown(netfn, _, _) => netfn % 2 == 1,
        }
    }

    pub fn is_response_for(&self, other: &Self) -> bool {
        let net_fn = self.netfn_id();
        let cmd = self.cmd_id();

        let other_net_fn = other.netfn_id();
        let other_cmd = other.cmd_id();

        (other_net_fn + 1 == net_fn) && cmd == other_cmd
    }
}
