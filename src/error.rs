use crate::connection::NetFn;

#[derive(Clone, Debug, PartialEq)]
pub enum IpmiError<CON, P> {
    NetFnIsResponse(NetFn),
    UnexpectedResponse {
        netfn_sent: NetFn,
        netfn_recvd: NetFn,
        cmd_sent: u8,
        cmd_recvd: u8,
    },
    ParsingFailed {
        error: P,
        netfn: NetFn,
        cmd: u8,
        completion_code: u8,
        data: Vec<u8>,
    },
    Connection(CON),
}

impl<CON, P> From<CON> for IpmiError<CON, P> {
    fn from(value: CON) -> Self {
        Self::Connection(value)
    }
}

impl<CON, P> IpmiError<CON, P> {
    pub fn map<CON2, F>(self, f: F) -> IpmiError<CON2, P>
    where
        F: FnOnce(CON) -> CON2,
    {
        match self {
            IpmiError::NetFnIsResponse(v) => IpmiError::NetFnIsResponse(v),
            IpmiError::UnexpectedResponse {
                netfn_sent,
                netfn_recvd,
                cmd_sent,
                cmd_recvd,
            } => IpmiError::UnexpectedResponse {
                netfn_sent,
                netfn_recvd,
                cmd_sent,
                cmd_recvd,
            },
            IpmiError::ParsingFailed {
                error,
                netfn,
                cmd,
                completion_code,
                data,
            } => IpmiError::ParsingFailed {
                error,
                netfn,
                cmd,
                completion_code,
                data,
            },
            IpmiError::Connection(e) => IpmiError::Connection(f(e)),
        }
    }
}
