use crate::connection::{EncodeIpmiCommand, IpmiCommand, NetFn, NotEnoughData};

#[derive(Clone, Copy, Debug)]
pub struct GetAllocInfo;

impl EncodeIpmiCommand for GetAllocInfo {
    const NETFN: NetFn = NetFn::Storage;
    const CMD: u8 = 0x21;

    fn request_data_len(&self) -> usize {
        0
    }

    fn write_request_data(&self, data: &mut [u8]) {
        debug_assert!(data.is_empty());
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
