use crate::bitmap::{Bitmap, BitmapLayout, decode_bitmap, encode_bitmap};
use crate::{Error, Identity, UnpackNibbles};

const ISO_2_LAYOUT: BitmapLayout = BitmapLayout::iso(1, 2);
type BitmapBinaryWord = Identity;
type BitmapAsciiHexWord = UnpackNibbles<crate::primitive::nibble::UpperHexDigits>;

#[inline(never)]
pub fn encode_bitmap_binary_iso2(output: &mut &mut [u8], scratch: &mut [u8], bitmap: &Bitmap) -> Result<(), Error> {
    encode_bitmap::<BitmapBinaryWord>(output, scratch, bitmap, ISO_2_LAYOUT)
}

#[inline(never)]
pub fn decode_bitmap_binary_iso2(input: &mut &[u8], scratch: &mut [u8]) -> Result<Bitmap, Error> {
    decode_bitmap::<BitmapBinaryWord>(input, scratch, ISO_2_LAYOUT)
}

#[inline(never)]
pub fn encode_bitmap_ascii_hex_iso2(output: &mut &mut [u8], scratch: &mut [u8], bitmap: &Bitmap) -> Result<(), Error> {
    encode_bitmap::<BitmapAsciiHexWord>(output, scratch, bitmap, ISO_2_LAYOUT)
}

#[inline(never)]
pub fn decode_bitmap_ascii_hex_iso2(input: &mut &[u8], scratch: &mut [u8]) -> Result<Bitmap, Error> {
    decode_bitmap::<BitmapAsciiHexWord>(input, scratch, ISO_2_LAYOUT)
}

#[inline(never)]
pub fn encode_bitmap_binary_iso2_required(output: &mut &mut [u8], scratch: &mut [u8], bitmap: &Bitmap) -> Result<(), Error> {
    encode_bitmap::<BitmapBinaryWord>(output, scratch, bitmap, BitmapLayout::iso(2, 2))
}

#[inline(never)]
pub fn decode_bitmap_binary_iso2_required(input: &mut &[u8], scratch: &mut [u8]) -> Result<Bitmap, Error> {
    decode_bitmap::<BitmapBinaryWord>(input, scratch, BitmapLayout::iso(2, 2))
}

#[inline(never)]
pub fn encode_bitmap_binary_fixed2(output: &mut &mut [u8], scratch: &mut [u8], bitmap: &Bitmap) -> Result<(), Error> {
    encode_bitmap::<BitmapBinaryWord>(output, scratch, bitmap, BitmapLayout::fixed(2))
}

#[inline(never)]
pub fn decode_bitmap_binary_fixed2(input: &mut &[u8], scratch: &mut [u8]) -> Result<Bitmap, Error> {
    decode_bitmap::<BitmapBinaryWord>(input, scratch, BitmapLayout::fixed(2))
}

#[inline(never)]
pub fn encode_bitmap_binary_half_word(output: &mut &mut [u8], scratch: &mut [u8], bitmap: &Bitmap) -> Result<(), Error> {
    encode_bitmap::<BitmapBinaryWord>(output, scratch, bitmap, BitmapLayout::bits(32))
}

#[inline(never)]
pub fn decode_bitmap_binary_half_word(input: &mut &[u8], scratch: &mut [u8]) -> Result<Bitmap, Error> {
    decode_bitmap::<BitmapBinaryWord>(input, scratch, BitmapLayout::bits(32))
}

#[inline(never)]
pub fn bitmap_get_field2(bitmap: &Bitmap) -> bool {
    bitmap.get(2)
}

#[inline(never)]
pub fn bitmap_set_field2(bitmap: &mut Bitmap, value: bool) {
    bitmap.set(2, value);
}

#[inline(never)]
pub fn bitmap_word1(bitmap: &Bitmap) -> u64 {
    bitmap.word(1)
}

#[inline(never)]
pub fn bitmap_set_word1(bitmap: &mut Bitmap, word: u64) {
    bitmap.set_word(1, word);
}

#[inline(never)]
pub fn bitmap_highest_word(bitmap: &Bitmap) -> usize {
    bitmap.highest_word()
}
