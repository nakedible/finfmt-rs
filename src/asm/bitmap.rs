use crate::primitive::bitmap::{Bitmap, BitmapLayout, decode_bitmap, encode_bitmap};
use crate::{Binary, Error, Field, Fixed, UnpackNibbles};

const ISO_2_LAYOUT: BitmapLayout = BitmapLayout::iso(2);
type BitmapBinaryWord = Field<Binary<8, 8>, Fixed<8>>;
type BitmapAsciiHexWord = Field<Binary<8, 8>, Fixed<8>, UnpackNibbles<crate::primitive::nibble::HexUpper>>;

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
