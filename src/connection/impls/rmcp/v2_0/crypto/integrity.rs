#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IntegrityAlgorithm {
    None,
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

impl TryFrom<u8> for IntegrityAlgorithm {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let value = match value {
            0x00 => Self::None,
            0x01 => Self::HmacSha1_96,
            0x02 => Self::HmacMd5_128,
            0x03 => Self::Md5_128,
            0x04 => Self::HmacSha256_128,
            _ => return Err(()),
        };

        Ok(value)
    }
}

impl From<IntegrityAlgorithm> for u8 {
    fn from(value: IntegrityAlgorithm) -> Self {
        match value {
            IntegrityAlgorithm::None => 0x00,
            IntegrityAlgorithm::HmacSha1_96 => 0x01,
            IntegrityAlgorithm::HmacMd5_128 => 0x02,
            IntegrityAlgorithm::Md5_128 => 0x03,
            IntegrityAlgorithm::HmacSha256_128 => 0x04,
        }
    }
}
