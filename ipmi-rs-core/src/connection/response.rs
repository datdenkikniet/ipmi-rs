use super::NetFn;

/// An IPMI response.
#[derive(Debug, Clone, PartialEq)]
pub struct Response {
    seq: i64,
    netfn: u8,
    cmd: u8,
    data: Vec<u8>,
}

impl Response {
    /// Create a new IPMI response message.
    ///
    /// Returns `None` if `netfn` is not a response code, or if
    /// `data` is empty.
    pub fn new(netfn: u8, cmd: u8, data: Vec<u8>, seq: i64) -> Option<Self> {
        let netfn_parsed = NetFn::from(netfn);

        if data.is_empty() || netfn_parsed.response_value() != netfn {
            None
        } else {
            Some(Self {
                netfn,
                cmd,
                data,
                seq,
            })
        }
    }

    /// Get the netfn for the request.
    pub fn netfn(&self) -> NetFn {
        self.netfn.into()
    }

    /// Get the raw value of the netfn for the request.
    pub fn netfn_raw(&self) -> u8 {
        self.netfn
    }

    /// Get the command value for the request.
    pub fn cmd(&self) -> u8 {
        self.cmd
    }

    /// Get the sequence number for the response.
    pub fn seq(&self) -> i64 {
        self.seq
    }

    /// Get the completion code for the response.
    pub fn cc(&self) -> u8 {
        self.data[0]
    }

    /// Get a shared reference to the data of the request (does not include netfn, command, or completion code).
    pub fn data(&self) -> &[u8] {
        &self.data[1..]
    }
}
