#[cfg(all(not(debug_assertions), feature = "no-panic"))]
use no_panic::no_panic;

use super::bytes::{is_filled, take_bytes};
use crate::utils::cold_path;
use crate::{Error, Step};

/// Presence bits for fields 1 through 192, stored most-significant bit first.
///
/// Word `k` holds fields `64k + 1` through `64k + 64`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Bitmap([u64; 3]);

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

impl Bitmap {
    #[inline(always)]
    pub const fn new() -> Self {
        Self([0; 3])
    }

    /// Set or clear a one-based field number in `1..=192`.
    #[inline(always)]
    #[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
    pub fn set(&mut self, id: u16, value: bool) {
        debug_assert!(id > 0 && id <= 192, "bitmap field id out of range");
        // Out-of-range ids, including 0 through wrapping, land past the words.
        let bit = usize::from(id.wrapping_sub(1));
        let (word, offset) = (bit / 64, bit % 64);
        let mask = 1u64 << (63 - offset);
        if let Some(slot) = self.0.get_mut(word) {
            if value {
                *slot |= mask;
            } else {
                *slot &= !mask;
            }
        }
    }

    /// Read a one-based field number in `1..=192`.
    #[inline(always)]
    #[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
    pub fn get(&self, id: u16) -> bool {
        debug_assert!(id > 0 && id <= 192, "bitmap field id out of range");
        // Out-of-range ids, including 0 through wrapping, land past the words.
        let bit = usize::from(id.wrapping_sub(1));
        let (word, offset) = (bit / 64, bit % 64);
        match self.0.get(word) {
            Some(value) => value & (1u64 << (63 - offset)) != 0,
            None => false,
        }
    }

    /// Read a raw word at a zero-based index in `0..3`.
    #[inline(always)]
    #[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
    pub fn word(&self, index: usize) -> u64 {
        debug_assert!(index < 3, "bitmap word index out of range");
        match self.0.get(index) {
            Some(word) => *word,
            None => {
                cold_path();
                0
            }
        }
    }

    /// Replace a raw word at a zero-based index in `0..3`.
    #[inline(always)]
    #[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
    pub fn set_word(&mut self, index: usize, word: u64) {
        debug_assert!(index < 3, "bitmap word index out of range");
        if let Some(slot) = self.0.get_mut(index) {
            *slot = word;
        } else {
            cold_path();
        }
    }

    /// Return the highest nonzero word index, or zero for an empty bitmap.
    #[inline(always)]
    #[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
    pub fn highest_word(&self) -> usize {
        self.0.iter().rposition(|&word| word != 0).unwrap_or(0)
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
    let wire = take_bytes(input, S::encoded_len_of_count(len)?)?;
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
