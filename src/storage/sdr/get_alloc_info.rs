use crate::{
    connection::{IpmiCommand, Message, NetFn, NotEnoughData},
    Loggable,
};

#[derive(Clone, Copy, Debug)]
pub struct GetAllocInfo;

impl From<GetAllocInfo> for Message {
    fn from(_: GetAllocInfo) -> Self {
        Message::new_request(NetFn::Storage, 0x21, Vec::new())
    }
}

impl IpmiCommand for GetAllocInfo {
    type Output = AllocInfo;

    type Error = NotEnoughData;

    fn parse_success_response(data: &[u8]) -> Result<Self::Output, Self::Error> {
        AllocInfo::parse(data).ok_or(NotEnoughData)
    }
}

#[derive(Clone, Debug)]
pub struct AllocInfo {
    inner: crate::storage::AllocInfo,
}

impl AllocInfo {
    pub fn parse(data: &[u8]) -> Option<Self> {
        Some(Self {
            inner: crate::storage::AllocInfo::parse(data)?,
        })
    }
}

impl Loggable for AllocInfo {
    fn as_log(&self) -> Vec<crate::fmt::LogItem> {
        let mut log = self.inner.as_log();
        log.insert(0, (0, "SDR Repository Allocation Information").into());
        log
    }
}

impl core::ops::Deref for AllocInfo {
    type Target = crate::storage::AllocInfo;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl core::ops::DerefMut for AllocInfo {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
