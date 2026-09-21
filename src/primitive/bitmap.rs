#[cfg(all(not(debug_assertions), feature = "no-panic"))]
use no_panic::no_panic;

use super::bytes::all_bytes_eq;
use crate::utils::cold_path;
use crate::{Error, ScalarFmt};

/// Presence bits for fields 1 through 192, stored most-significant bit first.
///
/// Field numbers are independent of wire width: each raw word covers 64 fields.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Bitmap([u64; 3]);

/// Bitmap word counts and one-based, MSB-first continuation positions.
///
/// A clear continuation bit ends the bitmap, provided `min_words` have been
/// read. With no continuation bit, the next configured word is mandatory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BitmapLayout {
    pub min_words: u8,
    pub max_words: u8,
    pub continuation_bits: [Option<u8>; 3],
}

/// A scalar codec for the semantic bytes of one bitmap word.
///
/// Encoding receives exactly [`Self::DECODED_BYTES`] high-order bytes of a
/// 64-field word. The codec must preserve those bytes, and successful decoding
/// must return exactly that byte count. Wire framing, validation, and cursor
/// advancement follow [`ScalarFmt`]. Scratch may hold the decoded bytes until
/// they are copied into the owned bitmap.
///
/// Wire size can differ: eight decoded bytes become sixteen hexadecimal wire bytes.
pub trait BitmapWord: ScalarFmt {
    /// Number of semantic bytes per word, in `1..=8`, before wire transforms.
    const DECODED_BYTES: usize;
}

impl BitmapLayout {
    /// Configure one to three words with custom continuation positions.
    ///
    /// Requires `1 <= min_words <= max_words <= 3`. Continuation positions
    /// must be in `1..=64`; entries beyond `max_words` do not affect framing.
    #[inline(always)]
    pub const fn new(min_words: u8, max_words: u8, continuation_bits: [Option<u8>; 3]) -> Self {
        Self {
            min_words,
            max_words,
            continuation_bits,
        }
    }

    /// Use ISO continuation bits with the given minimum and maximum word counts.
    #[inline(always)]
    pub const fn iso(min_words: u8, max_words: u8) -> Self {
        Self::new(min_words, max_words, [Some(1), Some(1), None])
    }

    /// Always encode and decode exactly `words` words, without continuation bits.
    #[inline(always)]
    pub const fn fixed(words: u8) -> Self {
        Self::new(words, words, [None; 3])
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
        if id == 0 || id > 192 {
            cold_path();
            return;
        }
        let bit = usize::from(id - 1);
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
        if id == 0 || id > 192 {
            cold_path();
            return false;
        }
        let bit = usize::from(id - 1);
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

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
fn validate_bitmap_layout<F: BitmapWord>(layout: BitmapLayout) -> Result<usize, Error> {
    debug_assert!(F::DECODED_BYTES > 0 && F::DECODED_BYTES <= 8, "bitmap word byte width out of range");
    if F::DECODED_BYTES == 0 || F::DECODED_BYTES > 8 {
        cold_path();
        return Err(Error::Internal);
    }
    let max_words = usize::from(layout.max_words);
    debug_assert!(max_words > 0 && max_words <= 3, "bitmap max_words out of range");
    if max_words == 0 || max_words > 3 {
        cold_path();
        return Err(Error::Internal);
    }
    debug_assert!(
        layout.min_words > 0 && layout.min_words <= layout.max_words,
        "bitmap min_words out of range"
    );
    if layout.min_words == 0 || layout.min_words > layout.max_words {
        cold_path();
        return Err(Error::Internal);
    }

    let mut index = 0usize;
    while index < 3 {
        if let Some(bit) = layout.continuation_bits[index] {
            debug_assert!(bit > 0 && bit <= 64, "bitmap continuation bit out of range");
            debug_assert!(
                index >= max_words || usize::from(bit) <= F::DECODED_BYTES * 8,
                "bitmap continuation bit outside word width"
            );
            if bit == 0 || bit > 64 {
                cold_path();
                return Err(Error::Internal);
            }
        }
        index += 1;
    }

    Ok(max_words)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
fn continuation_mask(layout: BitmapLayout, index: usize) -> u64 {
    debug_assert!(index < 3, "bitmap word index out of range");
    match layout.continuation_bits.get(index).copied().flatten() {
        Some(bit @ 1..=64) => 1u64 << (64 - bit),
        _ => 0,
    }
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
fn encode_bitmap_word<F: BitmapWord>(output: &mut &mut [u8], scratch: &mut [u8], word: u64) -> Result<(), Error> {
    let mut scratch_ptr = &mut scratch[..];
    let word = word.to_be_bytes();
    debug_assert!(
        word.get(F::DECODED_BYTES..).is_some_and(|tail| all_bytes_eq(tail, 0)),
        "bitmap contains bits outside word width"
    );
    let bytes = word.get(..F::DECODED_BYTES).ok_or_else(|| {
        cold_path();
        Error::Internal
    })?;
    F::encode(output, &mut scratch_ptr, bytes)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
fn decode_bitmap_word<F: BitmapWord>(input: &mut &[u8], scratch: &mut [u8]) -> Result<u64, Error> {
    // Reborrow input for scratch's lifetime, then restore a suffix of the original slice.
    let source = *input;
    let mut input_ptr = source;
    let mut scratch_ptr = &mut scratch[..];
    let bytes = F::decode(&mut input_ptr, &mut scratch_ptr)?;
    debug_assert_eq!(bytes.len(), F::DECODED_BYTES, "bitmap word decoder returned incorrect length");
    if bytes.len() != F::DECODED_BYTES {
        cold_path();
        return Err(Error::Internal);
    }
    let mut word = [0u8; 8];
    let dst = word.get_mut(..F::DECODED_BYTES).ok_or_else(|| {
        cold_path();
        Error::Internal
    })?;
    dst.copy_from_slice(bytes);
    let consumed = source.len().checked_sub(input_ptr.len()).ok_or_else(|| {
        cold_path();
        Error::Internal
    })?;
    *input = source.get(consumed..).ok_or_else(|| {
        cold_path();
        Error::Internal
    })?;
    Ok(u64::from_be_bytes(word))
}

/// Encode semantic presence bits through the supplied word format.
///
/// The bitmap must fit the layout, with all continuation positions clear and
/// no populated bits outside each word's decoded byte width. Short words use
/// the high bytes of each 64-field group: four-byte words cover fields 1–32,
/// 65–96, and 129–160. Active continuation positions must fit that width.
/// These caller/configuration preconditions are asserted in debug builds.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn encode_bitmap<F: BitmapWord>(
    output: &mut &mut [u8],
    scratch: &mut [u8],
    bitmap: &Bitmap,
    layout: BitmapLayout,
) -> Result<(), Error> {
    let max_words = validate_bitmap_layout::<F>(layout)?;
    debug_assert!(bitmap.highest_word() < max_words, "bitmap contains words outside layout");
    let mut highest_words = 1;
    for index in 0..max_words {
        if bitmap.word(index) != 0 {
            highest_words = index + 1;
        }
    }
    let required_words = highest_words.max(usize::from(layout.min_words));
    let mut words = max_words;
    for index in 0..max_words {
        if index + 1 >= required_words && continuation_mask(layout, index) != 0 {
            words = index + 1;
            break;
        }
    }
    for index in 0..words {
        let cont = continuation_mask(layout, index);
        debug_assert_eq!(bitmap.word(index) & cont, 0, "bitmap contains reserved continuation bits");
        let mut word = bitmap.word(index) & !cont;
        if index + 1 < words {
            word |= cont;
        }
        encode_bitmap_word::<F>(output, scratch, word)?;
    }
    Ok(())
}

/// Decode presence bits, removing continuation flags from the returned bitmap.
///
/// The word format and active continuation positions must agree on the decoded
/// width. Scratch is reused for each word; no borrow escapes into the bitmap.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn decode_bitmap<F: BitmapWord>(input: &mut &[u8], scratch: &mut [u8], layout: BitmapLayout) -> Result<Bitmap, Error> {
    let max_words = validate_bitmap_layout::<F>(layout)?;
    let mut bitmap = Bitmap::new();
    for index in 0..max_words {
        let word = decode_bitmap_word::<F>(input, scratch)?;
        let cont = continuation_mask(layout, index);
        bitmap.set_word(index, word & !cont);
        if cont != 0 {
            if word & cont == 0 {
                if index + 1 < usize::from(layout.min_words) {
                    cold_path();
                    return Err(Error::Invalid);
                }
                return Ok(bitmap);
            }
        } else if index + 1 == max_words {
            return Ok(bitmap);
        }
    }
    cold_path();
    Err(Error::Invalid)
}
