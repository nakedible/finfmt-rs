/// Constant wire bytes carried by a type, for formats that write and match a
/// fixed pattern: an absent value's encoding, for example.
///
/// Implement it on a marker type, or use a built-in carrier such as [`Fill`].
pub trait ConstBytes {
    const BYTES: &'static [u8];
}

/// `N` copies of `BYTE`, such as `Fill<0x40, 12>` for twelve EBCDIC spaces.
pub struct Fill<const BYTE: u8, const N: usize>;

impl<const BYTE: u8, const N: usize> Fill<BYTE, N> {
    const ARRAY: [u8; N] = [BYTE; N];
}

impl<const BYTE: u8, const N: usize> ConstBytes for Fill<BYTE, N> {
    const BYTES: &'static [u8] = &Self::ARRAY;
}

#[cfg(test)]
mod tests {
    use super::{ConstBytes, Fill};

    #[test]
    fn fill_repeats_its_byte() {
        assert_eq!(<Fill<0x40, 3>>::BYTES, [0x40; 3]);
        assert!(<Fill<b' ', 0>>::BYTES.is_empty());
    }
}
