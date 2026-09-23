use crate::Error;
use crate::primitive::nibble::{
    BcdzDigits, UpperHexDigits, pack_expanded_nibbles, pack_nibbles, unpack_nibbles, unpack_padded_nibbles, validate_nibbles,
};

#[inline(never)]
pub fn pack_nibbles_bcdz_left<'a>(output: &mut &'a mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
    pack_nibbles::<BcdzDigits>(output, input, false, 0x0F)
}

#[inline(never)]
pub fn pack_nibbles_bcdz_right<'a>(output: &mut &'a mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
    pack_nibbles::<BcdzDigits>(output, input, true, 0)
}

#[inline(never)]
pub fn pack_expanded_nibbles_hex_upper<'a>(output: &mut &'a mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
    pack_expanded_nibbles::<UpperHexDigits>(output, input)
}

#[inline(never)]
pub fn unpack_nibbles_bcdz<'a>(output: &mut &'a mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
    unpack_nibbles::<BcdzDigits>(output, input)
}

#[inline(never)]
pub fn unpack_padded_nibbles_bcdz_right<'a>(output: &mut &'a mut [u8], input: &[u8], output_len: usize) -> Result<&'a mut [u8], Error> {
    unpack_padded_nibbles::<BcdzDigits>(output, input, output_len, true, 0)
}

#[inline(never)]
pub fn unpack_padded_nibbles_bcdz_left<'a>(output: &mut &'a mut [u8], input: &[u8], output_len: usize) -> Result<&'a mut [u8], Error> {
    unpack_padded_nibbles::<BcdzDigits>(output, input, output_len, false, 0x0F)
}

#[inline(never)]
pub fn validate_nibbles_hex_upper(input: &[u8]) -> Result<(), Error> {
    validate_nibbles::<UpperHexDigits>(input)
}

#[inline(never)]
pub fn pack_nibbles_bcdz_left_15<'a>(output: &mut &'a mut [u8], input: &[u8; 15]) -> Result<&'a mut [u8], Error> {
    pack_nibbles::<BcdzDigits>(output, input, false, 0x0F)
}

#[inline(never)]
pub fn unpack_padded_nibbles_bcdz_left_15<'a>(output: &mut &'a mut [u8], input: &[u8; 8]) -> Result<&'a mut [u8], Error> {
    unpack_padded_nibbles::<BcdzDigits>(output, input, 15, false, 0x0F)
}

#[inline(never)]
pub fn pack_nibbles_bcdz_right_15<'a>(output: &mut &'a mut [u8], input: &[u8; 15]) -> Result<&'a mut [u8], Error> {
    pack_nibbles::<BcdzDigits>(output, input, true, 0)
}

#[inline(never)]
pub fn unpack_padded_nibbles_bcdz_right_15<'a>(output: &mut &'a mut [u8], input: &[u8; 8]) -> Result<&'a mut [u8], Error> {
    unpack_padded_nibbles::<BcdzDigits>(output, input, 15, true, 0)
}
