#[cfg(all(not(debug_assertions), feature = "no-panic"))]
use no_panic::no_panic;

use crate::utils::cold_path;

/// Presence bits for fields 1 through 192, stored most-significant bit first.
///
/// Word `k` holds fields `64k + 1` through `64k + 64`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Bitmap([u64; 3]);

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
