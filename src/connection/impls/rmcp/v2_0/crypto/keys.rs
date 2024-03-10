use aes::cipher::{consts::U16, generic_array::GenericArray};

use super::sha1::RunningHmac;

#[allow(unused)]
pub struct Keys {
    pub(super) sik: [u8; 20],
    pub(super) k1: [u8; 20],
    k2: [u8; 20],
    aes_key: GenericArray<u8, U16>,
    k3: [u8; 20],
}

impl core::fmt::Debug for Keys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Keys").finish()
    }
}

impl Keys {
    pub fn from_sik(sik: [u8; 20]) -> Self {
        let k2 = RunningHmac::new(&sik).feed(&[0x02; 20]).finalize();
        Self {
            sik,
            k1: RunningHmac::new(&sik).feed(&[0x01; 20]).finalize(),
            k2,
            k3: RunningHmac::new(&sik).feed(&[0x03; 20]).finalize(),
            aes_key: <[u8; 16]>::try_from(&k2[..16]).unwrap().into(),
        }
    }

    pub fn aes_key(&self) -> &GenericArray<u8, U16> {
        &self.aes_key
    }
}
