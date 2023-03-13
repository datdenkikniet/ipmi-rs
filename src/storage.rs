use crate::{
    connection::NetFns,
    fmt::{LogOutput, Loggable},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    GetSelInfo,
    GetSelAllocInfo,
    ReserveSel,
    PartialAddSelEntry,
    ClearSel,
    Unknown(u8),
}

impl core::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::GetSelInfo => write!(f, "Get SEL info"),
            Command::GetSelAllocInfo => write!(f, "Get SEL alloc info"),
            Command::ReserveSel => write!(f, "Reserve SEL"),
            Command::PartialAddSelEntry => write!(f, "Partial add SEL entry"),
            Command::ClearSel => write!(f, "Clear SEL"),
            Command::Unknown(v) => write!(f, "Unknown (0x{:02X}", v),
        }
    }
}

impl From<Command> for u8 {
    fn from(value: Command) -> Self {
        match value {
            Command::GetSelInfo => 0x40,
            Command::GetSelAllocInfo => 0x41,
            Command::ReserveSel => 0x42,
            Command::PartialAddSelEntry => 0x45,
            Command::ClearSel => 0x47,
            Command::Unknown(v) => v,
        }
    }
}

impl From<u8> for Command {
    fn from(value: u8) -> Self {
        match value {
            0x40 => Self::GetSelInfo,
            0x41 => Self::GetSelAllocInfo,
            0x42 => Self::ReserveSel,
            0x45 => Self::PartialAddSelEntry,
            0x47 => Self::ClearSel,
            v => Self::Unknown(v),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NetFn {
    is_response: bool,
    cmd: Command,
}

impl NetFn {
    const fn new(is_response: bool, cmd: Command) -> Self {
        Self { is_response, cmd }
    }
}

impl NetFns for NetFn {
    type Command = Command;

    fn request(cmd: Command) -> Self {
        Self::new(false, cmd)
    }

    fn new_response(cmd: Command) -> Self {
        Self::new(true, cmd)
    }

    fn is_response(&self) -> bool {
        self.is_response
    }

    fn cmd(&self) -> Command {
        self.cmd
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Timestamp(u32);

impl core::fmt::Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[cfg(feature = "time")]
        {
            let timestamp = time::OffsetDateTime::from_unix_timestamp(self.0 as i64).unwrap();

            let time = timestamp
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap();

            write!(f, "{}", time)
        }

        #[cfg(not(feature = "time"))]
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct SelInfo {
    pub version_maj: u8,
    pub version_min: u8,
    pub entries: u16,
    pub bytes_free: u16,
    pub last_add_time: Timestamp,
    pub last_del_time: Timestamp,
    pub overflow: bool,
    pub supported_cmds: Vec<Command>,
}

impl SelInfo {
    pub fn from_data(data: &[u8]) -> Option<Self> {
        if data.len() != 14 {
            return None;
        }

        let version_maj = data[0] & 0xF;
        let version_min = (data[0] >> 4) & 0xF;

        let entries = u16::from_le_bytes([data[1], data[2]]);
        let free = u16::from_le_bytes([data[3], data[4]]);

        let last_add_time = u32::from_le_bytes([data[5], data[6], data[7], data[8]]);
        let last_del_time = u32::from_le_bytes([data[9], data[10], data[11], data[12]]);
        let overflow = data[13] & 0x80 == 0x80;

        let mut supported_cmds = Vec::with_capacity(4);

        if data[13] & 0x08 == 0x08 {
            supported_cmds.push(Command::ClearSel);
        }
        if data[13] & 0x04 == 0x04 {
            supported_cmds.push(Command::PartialAddSelEntry);
        }
        if data[13] & 0x02 == 0x02 {
            supported_cmds.push(Command::ReserveSel);
        }
        if data[13] & 0x01 == 0x01 {
            supported_cmds.push(Command::GetSelAllocInfo);
        }

        Some(SelInfo {
            version_maj,
            version_min,
            entries,
            bytes_free: free,
            last_add_time: Timestamp(last_add_time),
            last_del_time: Timestamp(last_del_time),
            overflow,
            supported_cmds,
        })
    }
}

impl Loggable for SelInfo {
    fn log(&self, level: LogOutput) {
        use crate::log;
        let (ver_maj, ver_min) = (self.version_maj, self.version_min);

        log!(level, "SEL information:");
        log!(level, "  Version:        {}.{}", ver_maj, ver_min);
        log!(level, "  Entries:        {}", self.entries);
        log!(level, "  Bytes free:     {}", self.bytes_free);
        log!(level, "  Last addition:  {}", self.last_add_time);
        log!(level, "  Last erase:     {}", self.last_del_time);

        let supported_cmds: Vec<String> = self
            .supported_cmds
            .iter()
            .map(|cmd| format!("{}", cmd))
            .collect();

        log!(level, "  Supported cmds: {:?}", supported_cmds);
    }
}

#[derive(Debug, Clone)]
pub struct SelAllocInfo {
    pub num_alloc_units: u16,
    pub alloc_unit_size: u16,
    pub num_free_units: u16,
    pub largest_free_blk: u16,
    pub max_record_size: u8,
}

impl SelAllocInfo {
    pub fn from_data(data: &[u8]) -> Option<Self> {
        if data.len() < 8 {
            return None;
        }

        let num_alloc_units = u16::from_le_bytes([data[0], data[1]]);
        let alloc_unit_size = u16::from_le_bytes([data[2], data[3]]);
        let num_free_units = u16::from_le_bytes([data[4], data[5]]);
        let largest_free_blk = u16::from_le_bytes([data[6], data[7]]);
        let max_record_size = data[8];

        Some(Self {
            num_alloc_units,
            alloc_unit_size,
            num_free_units,
            largest_free_blk,
            max_record_size,
        })
    }
}

impl Loggable for SelAllocInfo {
    fn log(&self, level: LogOutput) {
        use crate::log;
        log!(level, "SEL Allocation info:");
        log!(level, "  # of units:         {}", self.num_alloc_units);
        log!(level, "  Unit size:          {}", self.alloc_unit_size);
        log!(level, "  # free units:       {}", self.num_free_units);
        log!(level, "  Largest free block: {}", self.largest_free_blk);
        log!(level, "  Max record size:    {}", self.max_record_size)
    }
}
