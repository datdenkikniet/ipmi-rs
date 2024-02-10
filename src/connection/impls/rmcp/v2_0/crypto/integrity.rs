use super::Algorithm;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IntegrityAlgorithm {
    HmacSha1_96,
    HmacMd5_128,
    Md5_128,
    HmacSha256_128,
}

impl Default for IntegrityAlgorithm {
    fn default() -> Self {
        Self::HmacSha1_96
    }
}

impl Algorithm for IntegrityAlgorithm {
    fn from_byte(value: u8) -> Result<Option<Self>, ()> {
        let value = match value {
            0x00 => return Ok(None),
            0x01 => Self::HmacSha1_96,
            0x02 => Self::HmacMd5_128,
            0x03 => Self::Md5_128,
            0x04 => Self::HmacSha256_128,
            _ => return Err(()),
        };

        Ok(Some(value))
    }

    fn into_byte(value: Option<Self>) -> u8 {
        match value {
            None => 0x00,
            Some(Self::HmacSha1_96) => 0x01,
            Some(Self::HmacMd5_128) => 0x02,
            Some(Self::Md5_128) => 0x03,
            Some(Self::HmacSha256_128) => 0x04,
        }
    }

    fn all() -> &'static [Self] {
        &[
            Self::HmacSha1_96,
            Self::HmacMd5_128,
            Self::Md5_128,
            Self::HmacSha256_128,
        ]
    }
}
