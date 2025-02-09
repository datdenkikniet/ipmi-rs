#![allow(missing_docs)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuthenticationAlgorithm {
    RakpNone,
    RakpHmacSha1,
    RakpHmacMd5,
    RakpHmacSha256,
}

impl From<AuthenticationAlgorithm> for u8 {
    fn from(value: AuthenticationAlgorithm) -> Self {
        match value {
            AuthenticationAlgorithm::RakpNone => 0x00,
            AuthenticationAlgorithm::RakpHmacSha1 => 0x01,
            AuthenticationAlgorithm::RakpHmacMd5 => 0x02,
            AuthenticationAlgorithm::RakpHmacSha256 => 0x03,
        }
    }
}

impl TryFrom<u8> for AuthenticationAlgorithm {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let value = match value {
            0x00 => Self::RakpNone,
            0x01 => Self::RakpHmacSha1,
            0x02 => Self::RakpHmacMd5,
            0x03 => Self::RakpHmacSha256,
            _ => return Err(()),
        };

        Ok(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConfidentialityAlgorithm {
    None,
    AesCbc128,
    Xrc4_128,
    Xrc4_40,
}

impl TryFrom<u8> for ConfidentialityAlgorithm {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let value = match value {
            0x00 => Self::None,
            0x01 => Self::AesCbc128,
            0x02 => Self::Xrc4_128,
            0x03 => Self::Xrc4_40,
            _ => return Err(()),
        };

        Ok(value)
    }
}

impl From<ConfidentialityAlgorithm> for u8 {
    fn from(value: ConfidentialityAlgorithm) -> Self {
        match value {
            ConfidentialityAlgorithm::None => 0x00,
            ConfidentialityAlgorithm::AesCbc128 => 0x01,
            ConfidentialityAlgorithm::Xrc4_128 => 0x02,
            ConfidentialityAlgorithm::Xrc4_40 => 0x03,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IntegrityAlgorithm {
    None,
    HmacSha1_96,
    HmacMd5_128,
    Md5_128,
    HmacSha256_128,
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
