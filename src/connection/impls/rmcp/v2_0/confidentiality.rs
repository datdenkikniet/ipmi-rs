#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConfidentialityAlgorithm {
    AesCbc128,
    Xrc4_128,
    Xrc4_40,
    Oem(u8),
}
