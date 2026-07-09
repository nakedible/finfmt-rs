use crate::Error;
use crate::primitive::nibble::{
    Bcdz, HexUpper, NibbleFormat, pack_expanded_nibbles, pack_nibbles, unpack_nibbles, unpack_padded_nibbles, validate_nibbles,
};

#[inline(never)]
pub fn pack_nibbles_bcdz_left<'a>(output: &mut &'a mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
    pack_nibbles(output, input, false, 0x0F, &Bcdz::TABLE)
}

#[inline(never)]
pub fn pack_nibbles_bcdz_right<'a>(output: &mut &'a mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
    pack_nibbles(output, input, true, 0, &Bcdz::TABLE)
}

#[inline(never)]
pub fn pack_expanded_nibbles_hex_upper<'a>(output: &mut &'a mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
    pack_expanded_nibbles(output, input, &HexUpper::TABLE)
}

#[inline(never)]
pub fn unpack_nibbles_bcdz<'a>(output: &mut &'a mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
    unpack_nibbles(output, input, &Bcdz::DIGITS)
}

#[inline(never)]
pub fn unpack_padded_nibbles_bcdz_right<'a>(output: &mut &'a mut [u8], input: &[u8], output_len: usize) -> Result<&'a mut [u8], Error> {
    unpack_padded_nibbles(output, input, output_len, true, 0, &Bcdz::DIGITS)
}

#[inline(never)]
pub fn unpack_padded_nibbles_bcdz_left<'a>(output: &mut &'a mut [u8], input: &[u8], output_len: usize) -> Result<&'a mut [u8], Error> {
    unpack_padded_nibbles(output, input, output_len, false, 0x0F, &Bcdz::DIGITS)
}

#[inline(never)]
pub fn validate_nibbles_hex_upper(input: &[u8]) -> Result<(), Error> {
    validate_nibbles(input, &HexUpper::TABLE)
}
