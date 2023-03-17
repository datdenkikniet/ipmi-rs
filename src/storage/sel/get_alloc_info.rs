use crate::{
    connection::{IpmiCommand, Message, NetFn},
    LogOutput, Loggable,
};

pub struct GetAllocInfo;

impl IpmiCommand for GetAllocInfo {
    type Output = AllocInfo;

    type Error = ();

    fn parse_response(
        completion_code: crate::connection::CompletionCode,
        data: &[u8],
    ) -> Result<Self::Output, crate::connection::ParseResponseError<Self::Error>> {
        Self::check_cc_success(completion_code)?;

        AllocInfo::from_data(data).ok_or(().into())
    }
}

impl Into<Message> for GetAllocInfo {
    fn into(self) -> Message {
        Message::new(NetFn::Storage, 0x41, Vec::new())
    }
}

#[derive(Debug, Clone)]
pub struct AllocInfo {
    pub num_alloc_units: u16,
    pub alloc_unit_size: u16,
    pub num_free_units: u16,
    pub largest_free_blk: u16,
    pub max_record_size: u8,
}

impl AllocInfo {
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

impl Loggable for AllocInfo {
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
