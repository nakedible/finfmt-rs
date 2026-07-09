#[cfg(all(not(debug_assertions), feature = "no-panic"))]
use no_panic::no_panic;

use crate::utils::cold_path;
use crate::{Error, ScalarFmt};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Bitmap([u64; 3]);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BitmapLayout {
    pub max_words: u8,
    pub continuation_bits: [Option<u8>; 3],
}

pub trait BitmapWord: ScalarFmt {
    const BYTES: usize;
}

impl BitmapLayout {
    #[inline(always)]
    pub const fn new(max_words: u8, continuation_bits: [Option<u8>; 3]) -> Self {
        Self {
            max_words,
            continuation_bits,
        }
    }

    #[inline(always)]
    pub const fn iso(max_words: u8) -> Self {
        Self::new(max_words, [Some(1), Some(1), None])
    }
}

impl Bitmap {
    #[inline(always)]
    pub const fn new() -> Self {
        Self([0; 3])
    }

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

    #[inline(always)]
    #[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
    pub fn highest_word(&self) -> usize {
        self.0.iter().rposition(|&word| word != 0).unwrap_or(0)
    }
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
fn validate_bitmap_layout(layout: BitmapLayout) -> Result<usize, Error> {
    let max_words = usize::from(layout.max_words);
    debug_assert!(max_words > 0 && max_words <= 3, "bitmap max_words out of range");
    if max_words == 0 || max_words > 3 {
        cold_path();
        return Err(Error::Internal);
    }

    let mut index = 0usize;
    while index < 3 {
        if let Some(bit) = layout.continuation_bits[index] {
            debug_assert!(bit > 0 && bit <= 64, "bitmap continuation bit out of range");
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
fn validate_bitmap_word<F: BitmapWord>() -> Result<(), Error> {
    debug_assert!(F::BYTES > 0 && F::BYTES <= 8, "bitmap word byte width out of range");
    if F::BYTES == 0 || F::BYTES > 8 {
        cold_path();
        return Err(Error::Internal);
    }
    Ok(())
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
fn encode_bitmap_word<F: BitmapWord>(output: &mut &mut [u8], scratch: &mut [u8], word: u64) -> Result<(), Error> {
    validate_bitmap_word::<F>()?;
    let mut scratch_ptr = &mut scratch[..];
    let word = word.to_be_bytes();
    let bytes = word.get(..F::BYTES).ok_or_else(|| {
        cold_path();
        Error::Internal
    })?;
    F::encode(output, &mut scratch_ptr, bytes)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
fn decode_bitmap_word<F: BitmapWord>(input: &mut &[u8], scratch: &mut [u8]) -> Result<u64, Error> {
    validate_bitmap_word::<F>()?;
    let source = *input;
    let mut input_ptr = source;
    let mut scratch_ptr = &mut scratch[..];
    let bytes = F::decode(&mut input_ptr, &mut scratch_ptr)?;
    if bytes.len() != F::BYTES {
        cold_path();
        return Err(Error::Invalid);
    }
    let mut word = [0u8; 8];
    let dst = word.get_mut(..F::BYTES).ok_or_else(|| {
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

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn encode_bitmap<F: BitmapWord>(
    output: &mut &mut [u8],
    scratch: &mut [u8],
    bitmap: &Bitmap,
    layout: BitmapLayout,
) -> Result<(), Error> {
    let max_words = validate_bitmap_layout(layout)?;
    let highest_words = bitmap.highest_word() + 1;
    debug_assert!(highest_words <= max_words, "bitmap contains words outside layout");
    if highest_words > max_words {
        cold_path();
        return Err(Error::Internal);
    }
    let has_continuation = layout.continuation_bits.iter().any(Option::is_some);
    let words = if has_continuation { highest_words } else { max_words };
    for index in 0..words {
        let mut word = bitmap.word(index) & !continuation_mask(layout, index);
        if index + 1 < words {
            let cont = continuation_mask(layout, index);
            if cont == 0 && has_continuation {
                cold_path();
                return Err(Error::Internal);
            }
            word |= cont;
        }
        encode_bitmap_word::<F>(output, scratch, word)?;
    }
    Ok(())
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn decode_bitmap<F: BitmapWord>(input: &mut &[u8], scratch: &mut [u8], layout: BitmapLayout) -> Result<Bitmap, Error> {
    let max_words = validate_bitmap_layout(layout)?;
    let mut bitmap = Bitmap::new();
    for index in 0..max_words {
        let word = decode_bitmap_word::<F>(input, scratch)?;
        let cont = continuation_mask(layout, index);
        bitmap.set_word(index, word & !cont);
        if cont != 0 {
            if word & cont == 0 {
                return Ok(bitmap);
            }
        } else if index + 1 == max_words {
            return Ok(bitmap);
        }
    }
    cold_path();
    Err(Error::Invalid)
}
