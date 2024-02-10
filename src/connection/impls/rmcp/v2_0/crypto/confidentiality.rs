use super::OptionalByteEquivalent;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConfidentialityAlgorithm {
    AesCbc128,
    Xrc4_128,
    Xrc4_40,
}

impl OptionalByteEquivalent for ConfidentialityAlgorithm {
    fn from_byte(value: u8) -> Result<Option<Self>, ()> {
        let value = match value {
            0x00 => return Ok(None),
            0x01 => Self::AesCbc128,
            0x02 => Self::Xrc4_128,
            0x03 => Self::Xrc4_40,
            _ => return Err(()),
        };

        Ok(Some(value))
    }

    fn into_byte(value: Option<Self>) -> u8 {
        match value {
            None => 0x00,
            Some(Self::AesCbc128) => 0x01,
            Some(Self::Xrc4_128) => 0x02,
            Some(Self::Xrc4_40) => 0x03,
        }
    }
}
