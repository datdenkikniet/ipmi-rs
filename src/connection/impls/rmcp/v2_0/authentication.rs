#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AuthenticationAlgorithm {
    RakpNone,
    RakpHmacSha1,
    RakpHmacMd5,
    RakpHmacSha256,
    Oem(u8),
}
