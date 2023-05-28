use crate::{
    connection::{IpmiCommand, Message, NetFn, ParseResponseError},
    Loggable,
};

pub struct GetAllocInfo;

impl IpmiCommand for GetAllocInfo {
    type Output = AllocInfo;

    type Error = ();

    fn parse_response(
        completion_code: crate::connection::CompletionCode,
        data: &[u8],
    ) -> Result<Self::Output, ParseResponseError<Self::Error>> {
        Self::check_cc_success(completion_code)?;

        AllocInfo::parse(data).ok_or(ParseResponseError::NotEnoughData)
    }
}

impl From<GetAllocInfo> for Message {
    fn from(_: GetAllocInfo) -> Self {
        Message::new_request(NetFn::Storage, 0x41, Vec::new())
    }
}

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
    fn into_log(&self) -> Vec<crate::fmt::LogItem> {
        let mut alloc_log_output = self.inner.into_log();
        alloc_log_output.insert(0, (0, "SEL Allocation Information").into());
        alloc_log_output
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
