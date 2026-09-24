use crate::Error;
use crate::primitive::text::{decode_padded, encode_padded, truncate_bytes, truncate_str};

#[inline(never)]
pub fn encode_padded_right_8_space<'a>(output: &mut &'a mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
    encode_padded(output, input, 8, false, b' ')
}

#[inline(never)]
pub fn encode_padded_left_8_space<'a>(output: &mut &'a mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
    encode_padded(output, input, 8, true, b' ')
}

#[inline(never)]
pub fn encode_padded_fixed_8_space<'a>(output: &mut &'a mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
    encode_padded(output, truncate_bytes(input, 8, false), 8, false, b' ')
}

#[inline(never)]
pub fn decode_padded_right_8_space(input: &[u8; 8]) -> Result<&[u8], Error> {
    Ok(decode_padded(input, 0, false, b' '))
}

#[inline(never)]
pub fn decode_padded_left_8_space(input: &[u8; 8]) -> Result<&[u8], Error> {
    Ok(decode_padded(input, 0, true, b' '))
}

#[inline(never)]
pub fn truncate_bytes_left_8(input: &[u8]) -> &[u8] {
    truncate_bytes(input, 8, false)
}

#[inline(never)]
pub fn truncate_bytes_right_8(input: &[u8]) -> &[u8] {
    truncate_bytes(input, 8, true)
}

#[inline(never)]
pub fn truncate_str_left_8(input: &str) -> &str {
    truncate_str(input, 8, false)
}

#[inline(never)]
pub fn truncate_str_right_8(input: &str) -> &str {
    truncate_str(input, 8, true)
}

#[inline(never)]
pub fn encode_truncated_ascii_4(output: &mut &mut [u8], input: &str) -> Result<(), Error> {
    use crate::{Ascii, Field, Fixed, ScalarFmt, Truncate};
    Truncate::<Field<Ascii<4, 4>, Fixed<4>>, 4>::encode_str(output, &mut [][..], input)
}

#[inline(never)]
pub fn decode_padded_protected_6(input: &[u8; 8]) -> Result<&[u8], Error> {
    Ok(decode_padded(input, 6, false, b' '))
}
