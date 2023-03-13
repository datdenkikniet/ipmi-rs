use crate::{
    app::DeviceId,
    connection::LogicalUnit,
    fmt::{LogOutput, Loggable},
    storage::{SelAllocInfo, SelInfo},
    AppCommand, NetFn, NetFns, StorageCommand,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Response {
    /// The NetFn of this response.
    ///
    /// Care should be take to ensure that [`net_fn.is_response()`] is always
    /// true. Currently it is ensure by a guard in [`Response::new`].
    ///
    /// [`net_fn.is_response()`]: NetFn::is_response
    net_fn: NetFn,
    seq: i64,
    lun: LogicalUnit,
    completion_code: u8,
    data: Vec<u8>,
}

impl Response {
    pub fn new(
        netfn: NetFn,
        seq: i64,
        lun: LogicalUnit,
        completion_code: u8,
        data: &[u8],
    ) -> Option<Self> {
        if netfn.is_response() {
            let mut data_vec = Vec::with_capacity(data.len());
            data_vec.extend_from_slice(data);

            Some(Self {
                net_fn: netfn,
                seq,
                lun,
                completion_code,
                data: data_vec,
            })
        } else {
            None
        }
    }

    pub fn netfn(&self) -> &NetFn {
        &self.net_fn
    }

    pub fn seq(&self) -> i64 {
        self.seq
    }

    pub fn lun(&self) -> LogicalUnit {
        self.lun
    }

    pub fn cc(&self) -> u8 {
        self.completion_code
    }

    pub fn data(&self) -> &Vec<u8> {
        &self.data
    }
}

#[derive(Debug, Clone)]
pub enum ParsedResponse {
    SelInfo(SelInfo),
    SelAllocInfo(SelAllocInfo),
    DeviceId(DeviceId),
}

impl Loggable for ParsedResponse {
    fn log(&self, output: LogOutput) {
        match self {
            ParsedResponse::SelInfo(sel_info) => sel_info.log(output),
            ParsedResponse::SelAllocInfo(sel_alloc_info) => sel_alloc_info.log(output),
            ParsedResponse::DeviceId(device_id) => device_id.log(output),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParseResponseError {
    Failed { completion_code: u8 },
    InvalidData,
    UnknownNetFn,
}

impl TryFrom<Response> for ParsedResponse {
    type Error = ParseResponseError;

    fn try_from(value: Response) -> Result<ParsedResponse, Self::Error> {
        if value.completion_code != 0 {
            return Err(ParseResponseError::Failed {
                completion_code: value.completion_code,
            });
        }

        match value.net_fn {
            NetFn::Storage(netfn) => match netfn.cmd() {
                StorageCommand::GetSelInfo => SelInfo::from_data(&value.data)
                    .map(Into::into)
                    .ok_or(ParseResponseError::InvalidData),
                StorageCommand::GetSelAllocInfo => SelAllocInfo::from_data(&value.data)
                    .map(Into::into)
                    .ok_or(ParseResponseError::InvalidData),
                StorageCommand::Unknown(_) => Err(ParseResponseError::UnknownNetFn),
                _ => unimplemented!(),
            },
            NetFn::App(netfn) => match netfn.cmd() {
                AppCommand::GetDeviceId => DeviceId::from_data(&value.data)
                    .map(Into::into)
                    .ok_or(ParseResponseError::InvalidData),
                AppCommand::Unknown(_) => Err(ParseResponseError::UnknownNetFn),
            },
            _ => Err(ParseResponseError::UnknownNetFn),
        }
    }
}

macro_rules! direct_from {
    ($($from:ty => $to:ident),*) => {
        $(
            impl From<$from> for ParsedResponse {
                fn from(value: $from) -> Self {
                    Self::$to(value)
                }
            }
        )*
    };
}

direct_from!(
    SelInfo => SelInfo,
    SelAllocInfo => SelAllocInfo,
    DeviceId => DeviceId
);
