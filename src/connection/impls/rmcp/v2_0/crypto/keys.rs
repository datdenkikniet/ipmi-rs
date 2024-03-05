use super::sha1::RunningHmac;

#[allow(unused)]
pub struct Keys {
    pub(super) sik: [u8; 20],
    pub(super) k1: [u8; 20],
    k2: [u8; 20],
    k3: [u8; 20],
}

impl core::fmt::Debug for Keys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Keys").finish()
    }
}

impl Keys {
    pub fn from_sik(sik: [u8; 20]) -> Self {
        Self {
            sik,
            k1: RunningHmac::new(&sik).feed(&[0x01; 20]).finalize(),
            k2: RunningHmac::new(&sik).feed(&[0x02; 20]).finalize(),
            k3: RunningHmac::new(&sik).feed(&[0x03; 20]).finalize(),
        }
    }
}

impl Default for Keys {
    fn default() -> Self {
        Self {
            sik: Default::default(),
            k1: Default::default(),
            k2: Default::default(),
            k3: Default::default(),
        }
    }
}
