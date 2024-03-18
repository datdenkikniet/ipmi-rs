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
