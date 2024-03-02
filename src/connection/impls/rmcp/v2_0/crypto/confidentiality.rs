#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConfidentialityAlgorithm {
    None,
    AesCbc128,
    Xrc4_128,
    Xrc4_40,
}

impl Default for ConfidentialityAlgorithm {
    fn default() -> Self {
        // TODO: default to AesCbc128
        Self::None
    }
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
