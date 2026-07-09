use crate::Error;
use crate::primitive::text::{decode_ascii, decode_bytes, encode_ascii, encode_bytes};

#[inline(never)]
pub fn encode_bytes_pad_right_8_space<'a>(output: &mut &'a mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
    encode_bytes(output, input, 8, usize::MAX, false, b' ')
}

#[inline(never)]
pub fn encode_bytes_pad_left_8_space<'a>(output: &mut &'a mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
    encode_bytes(output, input, 8, usize::MAX, true, b' ')
}

#[inline(never)]
pub fn encode_bytes_fixed_8_space<'a>(output: &mut &'a mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
    encode_bytes(output, input, 8, 8, false, b' ')
}

#[inline(never)]
pub fn decode_bytes_strip_right_8_space<'a>(input: &mut &'a [u8]) -> Result<&'a [u8], Error> {
    decode_bytes(input, 0, 8, false, b' ')
}

#[inline(never)]
pub fn decode_bytes_strip_left_8_space<'a>(input: &mut &'a [u8]) -> Result<&'a [u8], Error> {
    decode_bytes(input, 0, 8, true, b' ')
}

#[inline(never)]
pub fn encode_ascii_pad_right_8_space<'a>(output: &mut &'a mut [u8], input: &str) -> Result<&'a mut [u8], Error> {
    encode_ascii(output, input, 8, usize::MAX, false, b' ')
}

#[inline(never)]
pub fn encode_ascii_pad_left_8_space<'a>(output: &mut &'a mut [u8], input: &str) -> Result<&'a mut [u8], Error> {
    encode_ascii(output, input, 8, usize::MAX, true, b' ')
}

#[inline(never)]
pub fn decode_ascii_strip_right_8_space<'a>(input: &mut &'a [u8]) -> Result<&'a str, Error> {
    decode_ascii(input, 0, 8, false, b' ')
}

#[inline(never)]
pub fn decode_ascii_strip_left_8_space<'a>(input: &mut &'a [u8]) -> Result<&'a str, Error> {
    decode_ascii(input, 0, 8, true, b' ')
}
