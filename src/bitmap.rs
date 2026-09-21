pub use crate::primitive::bitmap::{Bitmap, BitmapLayout, BitmapWord, decode_bitmap, encode_bitmap};
use crate::{Binary, Field, Fixed, Step};

impl<S: Step, const N: usize> BitmapWord for Field<Binary<N, N>, Fixed<N>, S> {
    const BYTES: usize = N;
}

#[cfg(test)]
mod tests {
    use super::*;

    type BitmapBinaryHalfWord = crate::Field<crate::Binary<4, 4>, crate::Fixed<4>>;
    type BitmapBinaryWord = crate::Field<crate::Binary<8, 8>, crate::Fixed<8>>;
    type BitmapAsciiHexWord = crate::Field<crate::Binary<8, 8>, crate::Fixed<8>, crate::UnpackNibbles<crate::primitive::nibble::HexUpper>>;

    #[test]
    fn test_bitmap_bits_and_words() {
        let mut bitmap = Bitmap::new();
        bitmap.set(1, true);
        bitmap.set(64, true);
        bitmap.set(65, true);
        bitmap.set(192, true);
        assert!(bitmap.get(1) && bitmap.get(64) && bitmap.get(65) && bitmap.get(192));
        assert_eq!(bitmap.highest_word(), 2);
    }

    #[test]
    fn test_bitmap_ascii_hex_roundtrip() {
        let mut bitmap = Bitmap::new();
        bitmap.set(3, true);
        bitmap.set(63, true);
        bitmap.set(97, true);
        let layout = BitmapLayout::new(1, 3, [Some(64), Some(64), None]);
        let mut output = [0u8; 64];
        let mut scratch = [0u8; 8];
        let used = {
            let total = output.len();
            let mut out_ptr = output.as_mut_slice();
            encode_bitmap::<BitmapAsciiHexWord>(&mut out_ptr, &mut scratch, &bitmap, layout).unwrap();
            total - out_ptr.len()
        };
        let mut input = &output[..used];
        let decoded = decode_bitmap::<BitmapAsciiHexWord>(&mut input, &mut scratch, layout).unwrap();
        assert_eq!(decoded, bitmap);
        assert!(input.is_empty());
    }

    #[test]
    fn test_bitmap_ascii_hex_rejects_invalid_digits() {
        let layout = BitmapLayout::fixed(1);
        let mut input = b"000000000000000G".as_slice();
        let mut scratch = [0u8; 8];
        assert_eq!(
            decode_bitmap::<BitmapAsciiHexWord>(&mut input, &mut scratch, layout),
            Err(crate::Error::Invalid)
        );
    }

    #[test]
    fn test_bitmap_binary_half_word_roundtrip() {
        let mut bitmap = Bitmap::new();
        bitmap.set(2, true);
        bitmap.set(3, true);
        bitmap.set(4, true);
        let layout = BitmapLayout::fixed(1);
        let mut output = [0u8; 16];
        let mut scratch = [0u8; 8];
        let used = {
            let total = output.len();
            let mut out_ptr = output.as_mut_slice();
            encode_bitmap::<BitmapBinaryHalfWord>(&mut out_ptr, &mut scratch, &bitmap, layout).unwrap();
            total - out_ptr.len()
        };
        assert_eq!(&output[..used], &[0x70, 0x00, 0x00, 0x00]);
        let mut input = &output[..used];
        let decoded = decode_bitmap::<BitmapBinaryHalfWord>(&mut input, &mut scratch, layout).unwrap();
        assert_eq!(decoded, bitmap);
        assert!(input.is_empty());
    }

    #[test]
    fn test_bitmap_fixed_two_word_roundtrip() {
        let mut bitmap = Bitmap::new();
        bitmap.set(3, true);
        bitmap.set(65, true);
        let layout = BitmapLayout::fixed(2);
        let mut output = [0u8; 32];
        let mut scratch = [0u8; 8];
        let used = {
            let total = output.len();
            let mut out_ptr = output.as_mut_slice();
            encode_bitmap::<BitmapBinaryWord>(&mut out_ptr, &mut scratch, &bitmap, layout).unwrap();
            total - out_ptr.len()
        };
        let mut input = &output[..used];
        let decoded = decode_bitmap::<BitmapBinaryWord>(&mut input, &mut scratch, layout).unwrap();
        assert_eq!(decoded, bitmap);
        assert!(input.is_empty());
    }

    pub(super) fn roundtrip<F: BitmapWord>(bitmap: &Bitmap, layout: BitmapLayout, scratch_len: usize) -> ([u8; 49], usize) {
        let mut output = [0xA5; 49];
        let mut scratch = [0; 128];
        let mut out = &mut output[..48];
        encode_bitmap::<F>(&mut out, &mut scratch[..scratch_len], bitmap, layout).unwrap();
        let used = 48 - out.len();
        assert!(output[used..].iter().all(|&byte| byte == 0xA5));
        let mut input = &output[..used + 1];
        assert_eq!(decode_bitmap::<F>(&mut input, &mut scratch[..scratch_len], layout), Ok(*bitmap));
        assert_eq!(input, &[0xA5]);
        (output, used)
    }

    #[test]
    fn test_bitmap_required_optional_and_unflagged_words() {
        let flag = 1u64 << 63;
        let field = 1u64 << 62;
        let cases: &[(BitmapLayout, &[u16], &[u64])] = &[
            (BitmapLayout::iso(1, 2), &[], &[0]),
            (BitmapLayout::iso(1, 2), &[66], &[flag, field]),
            (BitmapLayout::iso(2, 2), &[], &[flag, 0]),
            (BitmapLayout::iso(2, 3), &[], &[flag, 0]),
            (BitmapLayout::iso(2, 3), &[130], &[flag, flag, field]),
            (BitmapLayout::iso(3, 3), &[], &[flag, flag, 0]),
            (BitmapLayout::fixed(2), &[], &[0, 0]),
            (BitmapLayout::fixed(2), &[1, 65], &[flag, flag]),
            (BitmapLayout::new(1, 3, [Some(1), None, None]), &[66], &[flag, field, 0]),
            (BitmapLayout::new(1, 3, [None, Some(1), None]), &[], &[0, 0]),
            (BitmapLayout::new(1, 2, [None, None, Some(1)]), &[], &[0, 0]),
            (BitmapLayout::new(2, 2, [Some(64), None, None]), &[], &[1, 0]),
        ];
        for &(layout, ids, expected) in cases {
            let mut bitmap = Bitmap::new();
            for &id in ids {
                bitmap.set(id, true);
            }
            let (wire, used) = roundtrip::<BitmapBinaryWord>(&bitmap, layout, 128);
            assert_eq!(used, expected.len() * 8);
            for (actual, word) in wire[..used].as_chunks::<8>().0.iter().zip(expected) {
                assert_eq!(actual, &word.to_be_bytes());
            }
            for capacity in 0..used {
                let mut output = [0; 24];
                assert_eq!(
                    encode_bitmap::<BitmapBinaryWord>(&mut &mut output[..capacity], &mut [], &bitmap, layout),
                    Err(crate::Error::BufferOverflow)
                );
            }
        }
    }

    #[test]
    fn test_bitmap_decode_required_words_and_extension_limits() {
        let mut wire = [0; 24];
        wire[0] = 0x80;
        wire[8] = 0x80;
        for min_words in 1..=3 {
            let layout = BitmapLayout::iso(min_words, 3);
            for len in 0..24 {
                assert_eq!(
                    decode_bitmap::<BitmapBinaryWord>(&mut &wire[..len], &mut [], layout),
                    Err(crate::Error::UnexpectedEof)
                );
            }
            let decoded = decode_bitmap::<BitmapBinaryWord>(&mut wire.as_slice(), &mut [], layout).unwrap();
            assert_eq!(decoded, Bitmap::new());
            assert_eq!(roundtrip::<BitmapBinaryWord>(&decoded, layout, 0).1, usize::from(min_words) * 8);
        }
        assert_eq!(
            decode_bitmap::<BitmapBinaryWord>(&mut wire.as_slice(), &mut [], BitmapLayout::iso(1, 2)),
            Err(crate::Error::Invalid)
        );
        wire[8] = 0;
        assert_eq!(
            decode_bitmap::<BitmapBinaryWord>(&mut wire.as_slice(), &mut [], BitmapLayout::iso(3, 3)),
            Err(crate::Error::Invalid)
        );
        wire[0] = 0;
        assert_eq!(
            decode_bitmap::<BitmapBinaryWord>(&mut wire.as_slice(), &mut [], BitmapLayout::iso(2, 2)),
            Err(crate::Error::Invalid)
        );
    }

    #[test]
    fn test_bitmap_transformed_words_reuse_scratch() {
        let mut bitmap = Bitmap::new();
        for id in [2, 66, 130, 192] {
            bitmap.set(id, true);
        }
        for scratch_len in [8, 128] {
            assert_eq!(roundtrip::<BitmapAsciiHexWord>(&bitmap, BitmapLayout::iso(1, 3), scratch_len).1, 48);
            assert_eq!(
                roundtrip::<BitmapAsciiHexWord>(&Bitmap::new(), BitmapLayout::iso(2, 2), scratch_len).1,
                32
            );
        }
        assert_eq!(
            decode_bitmap::<BitmapAsciiHexWord>(&mut b"0000000000000000".as_slice(), &mut [0; 7], BitmapLayout::fixed(1)),
            Err(crate::Error::BufferOverflow)
        );
    }

    #[test]
    fn test_bitmap_invalid_minimum() {
        for layout in [BitmapLayout::iso(0, 2), BitmapLayout::iso(3, 2)] {
            let encode = std::panic::catch_unwind(|| {
                encode_bitmap::<BitmapBinaryWord>(&mut [0; 24].as_mut_slice(), &mut [], &Bitmap::new(), layout)
            });
            let decode = std::panic::catch_unwind(|| decode_bitmap::<BitmapBinaryWord>(&mut [0; 24].as_slice(), &mut [], layout));
            if cfg!(debug_assertions) {
                assert!(encode.is_err() && decode.is_err());
            } else {
                assert_eq!(encode.unwrap(), Err(crate::Error::Internal));
                assert_eq!(decode.unwrap(), Err(crate::Error::Internal));
            }
        }
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "bitmap max_words out of range")]
    fn test_bitmap_invalid_layout_debug_asserts() {
        let bitmap = Bitmap::new();
        let layout = BitmapLayout::fixed(4);
        let mut output = [0u8; 32];
        let mut scratch = [0u8; 8];
        let mut out = output.as_mut_slice();
        let _ = encode_bitmap::<BitmapBinaryWord>(&mut out, &mut scratch, &bitmap, layout);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "bitmap contains words outside layout")]
    fn test_bitmap_outside_layout_debug_asserts() {
        let mut bitmap = Bitmap::new();
        bitmap.set(65, true);
        let layout = BitmapLayout::fixed(1);
        let mut output = [0u8; 32];
        let mut scratch = [0u8; 8];
        let mut out = output.as_mut_slice();
        let _ = encode_bitmap::<BitmapBinaryWord>(&mut out, &mut scratch, &bitmap, layout);
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn test_bitmap_invalid_inputs_return_errors_in_release() {
        let mut output = [0u8; 32];
        let mut scratch = [0u8; 8];
        let mut out = output.as_mut_slice();
        assert_eq!(
            encode_bitmap::<BitmapBinaryWord>(&mut out, &mut scratch, &Bitmap::new(), BitmapLayout::fixed(4)),
            Err(crate::Error::Internal)
        );

        let mut bitmap = Bitmap::new();
        bitmap.set(65, true);
        let mut out = output.as_mut_slice();
        assert_eq!(
            encode_bitmap::<BitmapBinaryWord>(&mut out, &mut scratch, &bitmap, BitmapLayout::fixed(1)),
            Err(crate::Error::Internal)
        );
    }
}

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[test]
        fn bitmap_layout_roundtrip(
            max_words in 1u8..=3,
            min_seed in 0u8..3,
            continuation_bits in prop::array::uniform3(prop::option::of(1u8..=64)),
            words in prop::array::uniform3(prop_oneof![Just(0u64), any::<u64>()]),
        ) {
            let layout = BitmapLayout::new(1 + min_seed % max_words, max_words, continuation_bits);
            let mut bitmap = Bitmap::new();
            for (index, &word) in words.iter().enumerate().take(usize::from(max_words)) {
                let reserved = continuation_bits[index].map_or(0, |bit| 1u64 << (64 - bit));
                bitmap.set_word(index, word & !reserved);
            }
            let (_, used) = super::tests::roundtrip::<Field<Binary<8, 8>, Fixed<8>>>(&bitmap, layout, 128);
            prop_assert!((usize::from(layout.min_words) * 8..=usize::from(max_words) * 8).contains(&used));
        }
    }
}
