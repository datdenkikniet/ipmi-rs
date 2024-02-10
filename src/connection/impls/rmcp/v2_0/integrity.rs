#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IntegrityAlgorithm {
    None,
    HmacSha1_96,
    HmacMd5_128,
    Md5_128,
    HmacSha256_128,
    Oem(u8),
}