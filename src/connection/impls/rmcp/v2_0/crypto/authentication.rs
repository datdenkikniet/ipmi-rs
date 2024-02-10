use super::Algorithm;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AuthenticationAlgorithm {
    RakpHmacSha1,
    RakpHmacMd5,
    RakpHmacSha256,
}

impl Default for AuthenticationAlgorithm {
    fn default() -> Self {
        Self::RakpHmacSha1
    }
}

impl Algorithm for AuthenticationAlgorithm {
    fn from_byte(value: u8) -> Result<Option<Self>, ()> {
        let value = match value {
            0x00 => return Ok(None),
            0x01 => Self::RakpHmacSha1,
            0x02 => Self::RakpHmacMd5,
            0x03 => Self::RakpHmacSha256,
            _ => return Err(()),
        };

        Ok(Some(value))
    }

    fn into_byte(value: Option<Self>) -> u8 {
        match value {
            None => 0x00,
            Some(Self::RakpHmacSha1) => 0x01,
            Some(Self::RakpHmacMd5) => 0x02,
            Some(Self::RakpHmacSha256) => 0x03,
        }
    }

    fn all() -> &'static [Self] {
        &[Self::RakpHmacSha1, Self::RakpHmacMd5, Self::RakpHmacSha256]
    }
}
