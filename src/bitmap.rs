//! Presence bitmaps: bit storage, word layouts, and their wire encoding.

#[cfg(all(not(debug_assertions), feature = "no-panic"))]
use no_panic::no_panic;

pub use crate::primitive::bitmap::Bitmap;
use crate::primitive::bytes::{is_filled, take_bytes};
use crate::utils::cold_path;
use crate::{Error, Step};

/// Bitmap word counts and the flags that announce the second and third words.
///
/// Words form a prefix: the second word may follow the first, and the third
/// the second. `word_flags[0]` and `word_flags[1]` flag the second and third
/// words as global field numbers in an earlier word, such as 1 and 65 in ISO
/// 8583. A word with a flag is present when that bit is set; a word without
/// one always follows its predecessor. The first `min_words` words are always
/// present, with their flags set. Flags of words within `max_words` are not
/// fields, so decoding clears them; other bits are ordinary fields.
///
/// Words are 64 bits, except a single-word bitmap made with [`Self::bits`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BitmapLayout {
    pub min_words: u8,
    pub max_words: u8,
    pub word_flags: [Option<u8>; 2],
    pub word_bits: u8,
}

impl BitmapLayout {
    /// Configure one to three words with custom word flags.
    ///
    /// Requires `1 <= min_words <= max_words <= 3`. The flag of word `k`
    /// (one-based) must be a distinct field number in words before it, so in
    /// `1..=64` for the second word and `1..=128` for the third. Flags of words
    /// beyond `max_words` are ignored.
    #[inline(always)]
    pub const fn new(min_words: u8, max_words: u8, word_flags: [Option<u8>; 2]) -> Self {
        Self {
            min_words,
            max_words,
            word_flags,
            word_bits: 64,
        }
    }

    /// Use the ISO 8583 flags, fields 1 and 65, with the given word counts.
    #[inline(always)]
    pub const fn iso(min_words: u8, max_words: u8) -> Self {
        Self::new(min_words, max_words, [Some(1), Some(65)])
    }

    /// Always encode and decode exactly `words` words, without flags.
    #[inline(always)]
    pub const fn fixed(words: u8) -> Self {
        Self::new(words, words, [None; 2])
    }

    /// A single word of `bits` bits, a multiple of 8 up to 64, holding fields
    /// 1 through `bits`.
    #[inline(always)]
    pub const fn bits(bits: u8) -> Self {
        Self {
            word_bits: bits,
            ..Self::fixed(1)
        }
    }

    /// Check the layout, returning what is wrong with it. `bitmap_format!`
    /// asserts this at compile time; the codecs assert it in debug builds.
    pub const fn validate(self) -> Result<(), &'static str> {
        if self.min_words == 0 || self.min_words > self.max_words || self.max_words > 3 {
            return Err("bitmap word counts must satisfy 1 <= min_words <= max_words <= 3");
        }
        if self.word_bits == 0 || self.word_bits > 64 || !self.word_bits.is_multiple_of(8) {
            return Err("bitmap word width must be a multiple of 8 bits up to 64");
        }
        if self.word_bits != 64 && self.max_words != 1 {
            return Err("a narrow bitmap must be a single word");
        }
        let [second, third] = self.word_flags;
        if self.max_words > 1
            && let Some(flag) = second
            && (flag == 0 || flag > 64)
        {
            return Err("the second bitmap word's flag must be in the first word");
        }
        if self.max_words > 2
            && let Some(flag) = third
        {
            if flag == 0 || flag > 128 {
                return Err("the third bitmap word's flag must be in an earlier word");
            }
            if let Some(second) = second
                && second == flag
            {
                return Err("bitmap word flags must be distinct");
            }
        }
        Ok(())
    }
}

/// The layout's word count, after asserting the layout in debug builds. A
/// malformed layout in release builds is clamped and may encode garbage.
#[inline(always)]
fn max_words(layout: BitmapLayout) -> usize {
    debug_assert_eq!(layout.validate(), Ok(()), "invalid bitmap layout");
    usize::from(layout.max_words).min(3)
}

/// The flag of zero-based word `index`, 1 or 2, if it has one.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
fn word_flag(layout: BitmapLayout, index: usize) -> Option<u8> {
    layout.word_flags.get(index.wrapping_sub(1)).copied().flatten()
}

/// The flags of the words within `max_words`, as bits.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
fn flag_bits(layout: BitmapLayout, max_words: usize) -> Bitmap {
    let mut flags = Bitmap::new();
    for index in 1..max_words {
        if let Some(flag) = word_flag(layout, index) {
            flags.set(u16::from(flag), true);
        }
    }
    flags
}

/// The bytes of each word, clamped so a malformed width cannot index past it.
#[inline(always)]
fn word_bytes(layout: BitmapLayout) -> usize {
    usize::from(layout.word_bits / 8).min(8)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
fn encode_bitmap_word<S: Step>(output: &mut &mut [u8], scratch: &mut [u8], word: u64, len: usize) -> Result<(), Error> {
    let word = word.to_be_bytes();
    debug_assert!(
        word.get(len..).is_some_and(|tail| is_filled(tail, 0)),
        "bitmap contains bits outside word width"
    );
    S::encode(output, scratch, word.get(..len).unwrap_or(&word))?;
    Ok(())
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
fn decode_bitmap_word<S: Step>(input: &mut &[u8], scratch: &mut [u8], len: usize) -> Result<u64, Error> {
    let wire = take_bytes(input, S::encoded_len(len)?)?;
    let bytes = S::decode(wire, &mut &mut scratch[..], Some(len))?;
    debug_assert_eq!(bytes.len(), len, "bitmap word step decoded a different number of bytes");
    let mut word = [0u8; 8];
    for (dst, &src) in word.iter_mut().zip(bytes) {
        *dst = src;
    }
    Ok(u64::from_be_bytes(word))
}

/// Encode semantic presence bits, each word through the representation step
/// `S`, such as `Identity` for binary or `UnpackNibbles` for hex.
///
/// Sends the words up to the last one with a field set, but at least
/// `min_words`, and any flagless words that must follow them, setting the flags
/// of the words sent. The bitmap must fit the layout and word width and must
/// not set word flags itself; these caller preconditions are asserted in debug
/// builds. A narrow word uses the high bytes of the first word.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn encode_bitmap<S: Step>(output: &mut &mut [u8], scratch: &mut [u8], bitmap: &Bitmap, layout: BitmapLayout) -> Result<(), Error> {
    let max_words = max_words(layout);
    debug_assert!(bitmap.highest_word() < max_words, "bitmap contains words outside layout");
    let flags = flag_bits(layout, max_words);
    let mut wire = Bitmap::new();
    let mut words = usize::from(layout.min_words).min(max_words);
    for index in 0..max_words {
        debug_assert_eq!(bitmap.word(index) & flags.word(index), 0, "bitmap sets a word flag");
        let word = bitmap.word(index) & !flags.word(index);
        if word != 0 {
            words = words.max(index + 1);
        }
        wire.set_word(index, word);
    }
    while words < max_words && word_flag(layout, words).is_none() {
        words += 1;
    }
    // Bounded by max_words, a constant for a constant layout, so this unrolls;
    // the flag bits are already clear, so setting them is branch-free.
    for index in 1..max_words {
        if let Some(flag) = word_flag(layout, index) {
            wire.set(u16::from(flag), index < words);
        }
    }
    for index in 0..words {
        encode_bitmap_word::<S>(output, scratch, wire.word(index), word_bytes(layout))?;
    }
    Ok(())
}

/// Decode presence bits, removing word flags from the returned bitmap.
///
/// A required word whose flag is clear, or a set flag for a word that did not
/// follow, is `Invalid`. Scratch is reused for each word; no borrow escapes
/// into the bitmap.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn decode_bitmap<S: Step>(input: &mut &[u8], scratch: &mut [u8], layout: BitmapLayout) -> Result<Bitmap, Error> {
    let max_words = max_words(layout);
    let mut bitmap = Bitmap::new();
    bitmap.set_word(0, decode_bitmap_word::<S>(input, scratch, word_bytes(layout))?);
    let mut words = 1;
    while words < max_words {
        if let Some(flag) = word_flag(layout, words)
            && !bitmap.get(u16::from(flag))
        {
            if words < usize::from(layout.min_words) {
                cold_path();
                return Err(Error::Invalid);
            }
            break;
        }
        bitmap.set_word(words, decode_bitmap_word::<S>(input, scratch, word_bytes(layout))?);
        words += 1;
    }
    let flags = flag_bits(layout, max_words);
    for index in words + 1..max_words {
        if let Some(flag) = word_flag(layout, index)
            && bitmap.get(u16::from(flag))
        {
            cold_path();
            return Err(Error::Invalid);
        }
    }
    for index in 0..max_words {
        bitmap.set_word(index, bitmap.word(index) & !flags.word(index));
    }
    Ok(bitmap)
}

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
    fn test_bitmap_ebcdic_hex_roundtrip() {
        type EbcdicHexWord = crate::UnpackNibbles<crate::primitive::nibble::EbcdicHexDigits>;
        let mut bitmap = Bitmap::new();
        bitmap.set(3, true);
        bitmap.set(66, true);
        let (wire, used) = roundtrip::<EbcdicHexWord>(&bitmap, BitmapLayout::iso(1, 2), 16);
        // A0... and 40...: fields 1 (the flag) and 3, then field 66.
        let mut expected = [0xF0; 32];
        expected[0] = 0xC1;
        expected[16] = 0xF4;
        assert_eq!(&wire[..used], &expected);
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
    fn test_bitmap_invalid_layouts_are_debug_errors() {
        let flags = |second, third| BitmapLayout::new(1, 3, [second, third]);
        for (layout, error) in [
            (BitmapLayout::iso(0, 2), "word counts"),
            (BitmapLayout::iso(3, 2), "word counts"),
            (BitmapLayout::fixed(4), "word counts"),
            (BitmapLayout::bits(12), "multiple of 8"),
            (BitmapLayout::bits(72), "multiple of 8"),
            (
                BitmapLayout {
                    max_words: 2,
                    ..BitmapLayout::bits(32)
                },
                "single word",
            ),
            (flags(Some(0), None), "second"),
            (flags(Some(65), None), "second"),
            (flags(None, Some(129)), "third"),
            (flags(Some(1), Some(1)), "distinct"),
        ] {
            assert!(layout.validate().is_err_and(|message| message.contains(error)), "{layout:?}");
            let encode = std::panic::catch_unwind(|| {
                encode_bitmap::<BitmapBinaryWord>(&mut [0; 24].as_mut_slice(), &mut [], &Bitmap::new(), layout)
            });
            let decode = std::panic::catch_unwind(|| decode_bitmap::<BitmapBinaryWord>(&mut [0; 24].as_slice(), &mut [], layout));
            assert_eq!(encode.is_err(), cfg!(debug_assertions));
            assert_eq!(decode.is_err(), cfg!(debug_assertions));
        }
        // Flags of words beyond max_words are ignored.
        assert_eq!(BitmapLayout::new(1, 2, [Some(1), Some(1)]).validate(), Ok(()));
        assert_eq!(BitmapLayout::new(1, 1, [Some(0), Some(200)]).validate(), Ok(()));
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

        fn decode<'a>(input: &'a [u8], scratch: &mut &'a mut [u8], _len: Option<usize>) -> Result<&'a [u8], crate::Error> {
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

    fn layouts() -> impl Strategy<Value = BitmapLayout> {
        prop_oneof![
            (1u8..=8).prop_map(|bytes| BitmapLayout::bits(bytes * 8)),
            (1u8..=3).prop_map(BitmapLayout::fixed),
            (1u8..=3, 0u8..3).prop_map(|(max, min)| BitmapLayout::iso(1 + min % max, max)),
            Just(BitmapLayout::new(1, 3, [Some(1), Some(2)])),
            Just(BitmapLayout::new(1, 3, [Some(1), None])),
        ]
    }

    /// A bitmap that fits the layout: fields within its words and width, no flags.
    fn fitting(layout: BitmapLayout, words: [u64; 3]) -> Bitmap {
        let mut bitmap = Bitmap::new();
        let width_mask = u64::MAX.checked_shl(64 - u32::from(layout.word_bits)).unwrap_or(0);
        for (index, &word) in words.iter().enumerate().take(usize::from(layout.max_words)) {
            bitmap.set_word(index, word & width_mask);
        }
        for flag in layout.word_flags.into_iter().take(usize::from(layout.max_words) - 1).flatten() {
            bitmap.set(u16::from(flag), false);
        }
        bitmap
    }

    proptest! {
        #[test]
        fn bitmap_words_roundtrip_in_every_representation(layout in layouts(), words in any::<[u64; 3]>()) {
            let bitmap = fitting(layout, words);
            super::tests::roundtrip::<crate::Identity>(&bitmap, layout, 0);
            super::tests::roundtrip::<crate::UnpackNibbles<crate::primitive::nibble::UpperHexDigits>>(&bitmap, layout, 16);
            super::tests::roundtrip::<crate::UnpackNibbles<crate::primitive::nibble::EbcdicHexDigits>>(&bitmap, layout, 16);
        }

        #[test]
        fn bitmap_decoding_arbitrary_input_is_stable(layout in layouts(), wire in prop::collection::vec(any::<u8>(), 0..32)) {
            // Never panics; whatever decodes re-encodes to something that decodes the same.
            let mut input = wire.as_slice();
            if let Ok(bitmap) = decode_bitmap::<crate::Identity>(&mut input, &mut [], layout) {
                let mut output = [0; 24];
                let mut out = output.as_mut_slice();
                encode_bitmap::<crate::Identity>(&mut out, &mut [], &bitmap, layout).unwrap();
                let used = 24 - out.len();
                prop_assert!(used <= wire.len() - input.len());
                prop_assert_eq!(decode_bitmap::<crate::Identity>(&mut &output[..used], &mut [], layout), Ok(bitmap));
            }
        }

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
