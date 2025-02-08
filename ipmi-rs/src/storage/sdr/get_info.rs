use crate::{
    connection::{IpmiCommand, Message, NetFn, NotEnoughData},
    fmt::LogItem,
    log_vec,
    storage::Timestamp,
    Loggable,
};

pub struct GetRepositoryInfo;

impl From<GetRepositoryInfo> for Message {
    fn from(_: GetRepositoryInfo) -> Self {
        Message::new_request(NetFn::Storage, 0x20, Vec::new())
    }
}

impl IpmiCommand for GetRepositoryInfo {
    type Output = RepositoryInfo;

    type Error = NotEnoughData;

    fn parse_success_response(data: &[u8]) -> Result<Self::Output, Self::Error> {
        RepositoryInfo::parse(data).ok_or(NotEnoughData)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FreeSpace {
    Full,
    AtLeast { bytes: u16 },
    Unspecified,
}

impl From<u16> for FreeSpace {
    fn from(value: u16) -> Self {
        match value {
            0x0000 => Self::Full,
            0xFFFF => Self::Unspecified,
            v => Self::AtLeast { bytes: v },
        }
    }
}

impl core::fmt::Display for FreeSpace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FreeSpace::Full => write!(f, "Full"),
            FreeSpace::AtLeast { bytes } => write!(f, "At least {} bytes", bytes),
            FreeSpace::Unspecified => write!(f, "Unspecified"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Operation {
    ModalityUnspecified,
    NonModalUpdate,
    ModalUpdate,
    NonModalAndModalUpdate,
    Delete,
    PartialAdd,
    Reserve,
    GetAllocInfo,
}

#[derive(Clone, Debug)]
pub struct RepositoryInfo {
    pub version_major: u8,
    pub version_minor: u8,
    pub record_count: u16,
    pub free_space: FreeSpace,
    pub most_recent_addition: Timestamp,
    pub most_recent_erase: Timestamp,
    pub overflow: bool,
    pub supported_ops: Vec<Operation>,
}

impl RepositoryInfo {
    pub fn parse(v: &[u8]) -> Option<Self> {
        let version_minor = (v[0] & 0xF0) >> 4;
        let version_major = v[0] & 0x0F;
        let record_count = u16::from_le_bytes([v[1], v[2]]);
        let free_space = FreeSpace::from(u16::from_le_bytes([v[3], v[4]]));
        let most_recent_addition = Timestamp::from(u32::from_le_bytes([v[5], v[6], v[7], v[8]]));
        let most_recent_erase = Timestamp::from(u32::from_le_bytes([v[9], v[10], v[11], v[12]]));
        let overflow = (v[13] & 0x80) == 0x80;

        let modality = v[13] & 0x60 >> 5;
        let modality = match modality {
            0b00 => Operation::ModalityUnspecified,
            0b01 => Operation::NonModalUpdate,
            0b10 => Operation::ModalUpdate,
            0b11 => Operation::NonModalAndModalUpdate,
            _ => unreachable!(),
        };

        let mut ops = Vec::with_capacity(5);
        ops.push(modality);

        for (offset, command) in [
            Operation::GetAllocInfo,
            Operation::Reserve,
            Operation::PartialAdd,
            Operation::Delete,
        ]
        .into_iter()
        .enumerate()
        {
            if v[13] & (1 << offset) == (1 << offset) {
                ops.push(command);
            }
        }

        Some(Self {
            version_major,
            version_minor,
            record_count,
            free_space,
            most_recent_addition,
            most_recent_erase,
            overflow,
            supported_ops: ops,
        })
    }
}

impl Loggable for RepositoryInfo {
    fn as_log(&self) -> Vec<LogItem> {
        let Self {
            version_major,
            version_minor,
            record_count,
            free_space,
            most_recent_addition,
            most_recent_erase,
            overflow,
            supported_ops,
        } = self;

        let (v_maj, v_min) = (version_major, version_minor);

        log_vec![
            (0, "SDR Repository Information"),
            (1, "Version", format!("{v_maj}.{v_min}")),
            (1, "Record count", record_count),
            (1, "Free space", free_space),
            (1, "Most recent add", most_recent_addition),
            (1, "Most recent erase", most_recent_erase),
            (1, "SDR Overflow", overflow),
            (1, "Supported ops", format!("{supported_ops:?}"))
        ]
    }
}
