pub use crate::primitive::bitmap::{Bitmap, BitmapLayout, decode_bitmap, encode_bitmap};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Step;

    type BitmapBinaryWord = crate::Identity;
    type BitmapAsciiHexWord = crate::UnpackNibbles<crate::primitive::nibble::UpperHexDigits>;

    #[test]
    fn test_bitmap_bits_and_words() {
        for id in 1..=192 {
            let mut bitmap = Bitmap::new();
            assert_eq!(bitmap.highest_word(), 0);
            bitmap.set(id, true);
            let index = usize::from((id - 1) / 64);
            let word = 1u64 << (63 - (id - 1) % 64);
            assert!(bitmap.get(id));
            assert_eq!(bitmap.word(index), word);
            assert_eq!(bitmap.highest_word(), index);
            bitmap.set(id, false);
            assert_eq!(bitmap, Bitmap::new());
            bitmap.set_word(index, word);
            assert!(bitmap.get(id));
            bitmap.set_word(index, 0);
            assert_eq!(bitmap, Bitmap::new());
        }
    }

    #[test]
    fn test_bitmap_ascii_hex_roundtrip() {
        let mut bitmap = Bitmap::new();
        bitmap.set(3, true);
        bitmap.set(63, true);
        bitmap.set(97, true);
        let layout = BitmapLayout::new(1, 3, [Some(64), Some(128)]);
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
        let layout = BitmapLayout::bits(32);
        let mut output = [0u8; 16];
        let mut scratch = [0u8; 8];
        let used = {
            let total = output.len();
            let mut out_ptr = output.as_mut_slice();
            encode_bitmap::<BitmapBinaryWord>(&mut out_ptr, &mut scratch, &bitmap, layout).unwrap();
            total - out_ptr.len()
        };
        assert_eq!(&output[..used], &[0x70, 0x00, 0x00, 0x00]);
        let mut input = &output[..used];
        let decoded = decode_bitmap::<BitmapBinaryWord>(&mut input, &mut scratch, layout).unwrap();
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

    pub(super) fn roundtrip<S: Step>(bitmap: &Bitmap, layout: BitmapLayout, scratch_len: usize) -> ([u8; 49], usize) {
        let mut output = [0xA5; 49];
        let mut scratch = [0; 128];
        let mut out = &mut output[..48];
        encode_bitmap::<S>(&mut out, &mut scratch[..scratch_len], bitmap, layout).unwrap();
        let used = 48 - out.len();
        assert!(output[used..].iter().all(|&byte| byte == 0xA5));
        let mut input = &output[..used + 1];
        assert_eq!(decode_bitmap::<S>(&mut input, &mut scratch[..scratch_len], layout), Ok(*bitmap));
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
            (BitmapLayout::new(1, 3, [Some(1), None]), &[66], &[flag, field, 0]),
            (BitmapLayout::new(1, 3, [None, Some(65)]), &[], &[0, 0]),
            (BitmapLayout::new(1, 2, [None, Some(1)]), &[], &[0, 0]),
            (BitmapLayout::new(2, 2, [Some(64), None]), &[], &[1, 0]),
            // Only flags of configured words are reserved: 65 is a field here.
            (BitmapLayout::iso(1, 2), &[65], &[flag, flag]),
            // Both flags in the first word; the third word still needs the second.
            (BitmapLayout::new(1, 3, [Some(1), Some(2)]), &[], &[0]),
            (BitmapLayout::new(1, 3, [Some(1), Some(2)]), &[66], &[flag, field]),
            (BitmapLayout::new(1, 3, [Some(1), Some(2)]), &[130], &[flag | field, 0, field]),
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
        let mut field_65 = Bitmap::new();
        field_65.set(65, true);
        assert_eq!(
            decode_bitmap::<BitmapBinaryWord>(&mut wire.as_slice(), &mut [], BitmapLayout::iso(1, 2)),
            Ok(field_65)
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
    fn test_bitmap_flag_for_a_word_that_did_not_follow_is_invalid() {
        let layout = BitmapLayout::new(1, 3, [Some(1), Some(2)]);
        let mut wire = [0; 24];
        wire[0] = 0x40;
        assert_eq!(
            decode_bitmap::<BitmapBinaryWord>(&mut wire.as_slice(), &mut [], layout),
            Err(crate::Error::Invalid)
        );
        wire[0] = 0xC0;
        let mut expected = Bitmap::new();
        expected.set(130, true);
        wire[16] = 0x40;
        assert_eq!(
            decode_bitmap::<BitmapBinaryWord>(&mut wire.as_slice(), &mut [], layout),
            Ok(expected)
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

    /// A binary word step that decodes into `N` scratch bytes.
    struct ScratchWord<const N: usize>;

    impl<const N: usize> Step for ScratchWord<N> {
        fn encoded_len(input_len: usize) -> Result<usize, crate::Error> {
            Ok(input_len)
        }

        fn encode<'a>(output: &mut &'a mut [u8], scratch: &mut [u8], input: &[u8]) -> Result<&'a mut [u8], crate::Error> {
            crate::Identity::encode(output, scratch, input)
        }

        fn decode<'a>(input: &'a [u8], scratch: &mut &'a mut [u8], _count: Option<usize>) -> Result<&'a [u8], crate::Error> {
            let out = crate::primitive::bytes::reserve_bytes(scratch, N)?;
            for (dst, &src) in out.iter_mut().zip(input.iter().cycle()) {
                *dst = src;
            }
            Ok(out)
        }
    }

    #[test]
    fn test_bitmap_custom_word_decode_contract() {
        fn wrong_width<const N: usize>() {
            let result = std::panic::catch_unwind(|| {
                decode_bitmap::<ScratchWord<N>>(&mut [0; 16].as_slice(), &mut [0; 128], BitmapLayout::fixed(1))
            });
            assert_eq!(result.is_err(), cfg!(debug_assertions));
        }
        wrong_width::<0>();
        wrong_width::<7>();
        wrong_width::<9>();

        let mut bitmap = Bitmap::new();
        for id in [2, 66, 130, 192] {
            bitmap.set(id, true);
        }
        for scratch_len in [8, 128] {
            assert_eq!(roundtrip::<ScratchWord<8>>(&bitmap, BitmapLayout::iso(1, 3), scratch_len).1, 24);
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

    #[test]
    fn test_bitmap_representability_diagnostics() {
        fn diagnosed<S: Step>(id: u16, layout: BitmapLayout) {
            let result = std::panic::catch_unwind(|| {
                let mut bitmap = Bitmap::new();
                bitmap.set(id, true);
                encode_bitmap::<S>(&mut [0; 24].as_mut_slice(), &mut [], &bitmap, layout)
            });
            assert_eq!(result.is_err(), cfg!(debug_assertions));
        }
        diagnosed::<BitmapBinaryWord>(33, BitmapLayout::bits(32));
        diagnosed::<BitmapBinaryWord>(
            2,
            BitmapLayout {
                max_words: 2,
                ..BitmapLayout::bits(32)
            },
        );
        diagnosed::<BitmapBinaryWord>(2, BitmapLayout::bits(12));
        diagnosed::<BitmapBinaryWord>(65, BitmapLayout::fixed(1));
        diagnosed::<BitmapBinaryWord>(1, BitmapLayout::iso(1, 2));
        diagnosed::<BitmapBinaryWord>(64, BitmapLayout::new(1, 2, [Some(64), None]));
        diagnosed::<BitmapBinaryWord>(2, BitmapLayout::new(1, 3, [Some(1), Some(1)]));
        diagnosed::<BitmapBinaryWord>(2, BitmapLayout::new(1, 2, [Some(65), None]));
        let decode = std::panic::catch_unwind(|| {
            decode_bitmap::<BitmapBinaryWord>(
                &mut [0; 8].as_slice(),
                &mut [],
                BitmapLayout {
                    word_bits: 32,
                    ..BitmapLayout::iso(1, 2)
                },
            )
        });
        assert_eq!(decode.is_err(), cfg!(debug_assertions));
    }

    #[test]
    fn test_bitmap_narrow_word_boundaries() {
        for bytes in 1..=8u8 {
            let mut bitmap = Bitmap::new();
            bitmap.set(u16::from(bytes) * 8, true);
            assert_eq!(
                roundtrip::<BitmapBinaryWord>(&bitmap, BitmapLayout::bits(bytes * 8), 128).1,
                usize::from(bytes)
            );
        }
        let mut bitmap = Bitmap::new();
        for index in 0..3 {
            bitmap.set(index * 64 + 64, true);
        }
        for layout in [BitmapLayout::fixed(3), BitmapLayout::iso(1, 3), BitmapLayout::iso(3, 3)] {
            assert_eq!(roundtrip::<BitmapBinaryWord>(&bitmap, layout, 128).1, 24);
        }
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn test_bitmap_malformed_parameters_do_not_panic() {
        for word_bits in [0, 8, 12, 32, 64, 72, 255] {
            for min in [0, 1, 2, 3, 4, 255] {
                for max in [0, 1, 2, 3, 4, 255] {
                    for bit in [None, Some(0), Some(1), Some(64), Some(65), Some(128), Some(193), Some(255)] {
                        let layout = BitmapLayout {
                            word_bits,
                            ..BitmapLayout::new(min, max, [bit; 2])
                        };
                        for capacity in [0, 1, 7, 8, 16, 24] {
                            let mut bitmap = Bitmap::new();
                            for index in 0..3 {
                                bitmap.set_word(index, u64::MAX);
                            }
                            let _ = encode_bitmap::<BitmapBinaryWord>(&mut &mut [0; 24][..capacity], &mut [], &bitmap, layout);
                            let _ = decode_bitmap::<BitmapBinaryWord>(&mut &[255; 24][..capacity], &mut [], layout);
                        }
                    }
                }
            }
        }
        let mut bitmap = Bitmap::new();
        for id in [0, 193, u16::MAX] {
            bitmap.set(id, true);
            assert!(!bitmap.get(id));
        }
        for index in [3, usize::MAX] {
            bitmap.set_word(index, u64::MAX);
            assert_eq!(bitmap.word(index), 0);
        }
        assert_eq!(bitmap, Bitmap::new());
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
            second_flag in prop::option::of(1u8..=64),
            third_flag in prop::option::of(1u8..=128),
            words in prop::array::uniform3(prop_oneof![Just(0u64), any::<u64>()]),
        ) {
            prop_assume!(max_words < 3 || second_flag.is_none() || second_flag != third_flag);
            let layout = BitmapLayout::new(1 + min_seed % max_words, max_words, [second_flag, third_flag]);
            let flags = [second_flag, third_flag].into_iter().take(usize::from(max_words) - 1).flatten();
            let mut bitmap = Bitmap::new();
            for (index, &word) in words.iter().enumerate().take(usize::from(max_words)) {
                bitmap.set_word(index, word);
            }
            for flag in flags {
                bitmap.set(u16::from(flag), false);
            }
            let (_, used) = super::tests::roundtrip::<crate::Identity>(&bitmap, layout, 128);
            prop_assert!((usize::from(layout.min_words) * 8..=usize::from(max_words) * 8).contains(&used));
        }
    }
}
