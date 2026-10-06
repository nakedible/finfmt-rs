/// Constant wire bytes carried by a type, for formats that write and match a
/// fixed pattern: an absent value's encoding, for example.
///
/// Implement it on a marker type, or use a built-in carrier such as [`Fill`].
pub trait ConstBytes {
    const BYTES: &'static [u8];
}

/// `N + M` copies of `BYTE`, such as `Fill<0x40, 12>` for twelve EBCDIC
/// spaces. As an [`AbsentFmt`](crate::AbsentFmt) it is a blank or zero-filled
/// field; `M` lets a generic alias add two widths, which stable Rust cannot do
/// in a type: a length prefix of `L` digits and a slot of `P` bytes, all blank
/// when absent, is `Fill<0x40, L, P>`. As [`ConstBytes`] it carries `N` bytes
/// and `M` is 0.
pub struct Fill<const BYTE: u8, const N: usize, const M: usize = 0>;

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
