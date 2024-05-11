use hmac::{Hmac, Mac};
use sha1::Sha1;

pub struct Sha1Hmac {
    state: Hmac<Sha1>,
}

impl Sha1Hmac {
    pub fn new(key: &[u8]) -> Self {
        Self {
            state: Hmac::new_from_slice(key)
                .expect("SHA1 HMAC initialization from bytes is infallible"),
        }
    }

    pub fn feed(mut self, data: &[u8]) -> Self {
        self.state.update(data);
        self
    }

    pub fn finalize(self) -> [u8; 20] {
        self.state.finalize().into_bytes().into()
    }
}
