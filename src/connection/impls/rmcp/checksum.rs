pub struct Checksum {
    state: u8,
}

impl Default for Checksum {
    fn default() -> Self {
        Self::new()
    }
}

impl Checksum {
    pub fn new() -> Self {
        Self { state: 0 }
    }

    pub fn from_iter(data: impl Iterator<Item = u8>) -> u8 {
        let mut me = Self::default();

        data.for_each(|v| me.feed(v));

        me.finalize()
    }

    pub fn feed(&mut self, data: u8) {
        self.state = self.state.wrapping_add(data);
    }

    pub fn finalize(&self) -> u8 {
        (!self.state).wrapping_add(1)
    }
}

#[test]
pub fn checksum_test() {
    let output = Checksum::from_iter([0x20, 0x06 << 2].into_iter());

    assert_eq!(0xC8, output);
}
