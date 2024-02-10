use std::iter::FusedIterator;

pub fn checksum(data: impl IntoIterator<Item = u8>) -> impl Iterator<Item = u8> + FusedIterator {
    struct ChecksumIterator<I> {
        checksum: u8,
        yielded_checksum: bool,
        inner: I,
    }

    impl<I: Iterator<Item = u8>> Iterator for ChecksumIterator<I> {
        type Item = u8;

        fn next(&mut self) -> Option<Self::Item> {
            if let Some(value) = self.inner.next() {
                self.checksum = self.checksum.wrapping_add(value);
                Some(value)
            } else if !self.yielded_checksum {
                self.yielded_checksum = true;
                self.checksum = !self.checksum;
                self.checksum = self.checksum.wrapping_add(1);
                Some(self.checksum)
            } else {
                None
            }
        }
    }

    impl<I: Iterator<Item = u8>> FusedIterator for ChecksumIterator<I> {}

    ChecksumIterator {
        checksum: 0,
        yielded_checksum: false,
        inner: data.into_iter(),
    }
}

#[test]
pub fn checksum_test() {
    let _output: Vec<_> = checksum([0x20, 0x06 << 2]).collect();
}
