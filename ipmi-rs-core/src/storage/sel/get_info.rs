use crate::{
    connection::{IpmiCommand, Message, NetFn, NotEnoughData},
    storage::Timestamp,
};

pub struct GetInfo;

impl IpmiCommand for GetInfo {
    type Output = Info;

    type Error = NotEnoughData;

    fn parse_success_response(data: &[u8]) -> Result<Self::Output, Self::Error> {
        Info::from_data(data).ok_or(NotEnoughData)
    }
}

impl From<GetInfo> for Message {
    fn from(_: GetInfo) -> Self {
        Message::new_request(NetFn::Storage, 0x40, Vec::new())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    Clear,
    PartialAddEntry,
    Reserve,
    GetAllocInfo,
}

#[derive(Debug, Clone)]
pub struct Info {
    pub version_maj: u8,
    pub version_min: u8,
    pub entries: u16,
    pub bytes_free: u16,
    pub last_add_time: Timestamp,
    pub last_del_time: Timestamp,
    pub overflow: bool,
    pub supported_cmds: Vec<Command>,
}

impl Info {
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
            supported_cmds.push(Command::Clear);
        }
        if data[13] & 0x04 == 0x04 {
            supported_cmds.push(Command::PartialAddEntry);
        }
        if data[13] & 0x02 == 0x02 {
            supported_cmds.push(Command::Reserve);
        }
        if data[13] & 0x01 == 0x01 {
            supported_cmds.push(Command::GetAllocInfo);
        }

        Some(Info {
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
