use crate::Error;
use crate::primitive::text::{decode_ascii, decode_bytes, encode_ascii, encode_bytes, truncate_bytes};

#[inline(never)]
pub fn encode_bytes_pad_right_8_space<'a>(output: &mut &'a mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
    encode_bytes(output, input, 8, false, b' ')
}

#[inline(never)]
pub fn encode_bytes_pad_left_8_space<'a>(output: &mut &'a mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
    encode_bytes(output, input, 8, true, b' ')
}

#[inline(never)]
pub fn encode_bytes_fixed_8_space<'a>(output: &mut &'a mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
    encode_bytes(output, truncate_bytes(input, 8, false), 8, false, b' ')
}

#[inline(never)]
pub fn decode_bytes_strip_right_8_space(input: &[u8; 8]) -> Result<&[u8], Error> {
    Ok(decode_bytes(input, 0, false, b' '))
}

#[inline(never)]
pub fn decode_bytes_strip_left_8_space(input: &[u8; 8]) -> Result<&[u8], Error> {
    Ok(decode_bytes(input, 0, true, b' '))
}

#[inline(never)]
pub fn encode_ascii_pad_right_8_space<'a>(output: &mut &'a mut [u8], input: &str) -> Result<&'a mut [u8], Error> {
    encode_ascii(output, input, 8, false, b' ')
}

#[inline(never)]
pub fn encode_ascii_pad_left_8_space<'a>(output: &mut &'a mut [u8], input: &str) -> Result<&'a mut [u8], Error> {
    encode_ascii(output, input, 8, true, b' ')
}

#[inline(never)]
pub fn decode_ascii_strip_right_8_space(input: &[u8; 8]) -> Result<&str, Error> {
    decode_ascii(input, 0, false, b' ')
}

#[inline(never)]
pub fn decode_ascii_strip_left_8_space(input: &[u8; 8]) -> Result<&str, Error> {
    decode_ascii(input, 0, true, b' ')
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
pub fn encode_truncated_ascii_4(output: &mut &mut [u8], input: &str) -> Result<(), Error> {
    use crate::{Ascii, Field, Fixed, ScalarFmt, TruncateBytes};
    TruncateBytes::<Field<Ascii<4, 4>, Fixed<4>>, 4>::encode_str(output, &mut &mut [][..], input)
}

#[inline(never)]
pub fn decode_bytes_protected_6(input: &[u8; 8]) -> Result<&[u8], Error> {
    Ok(decode_bytes(input, 6, false, b' '))
}
