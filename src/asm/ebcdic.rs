use crate::Error;
use crate::primitive::ebcdic::{
    ASCII_TO_EBCDIC_037, EBCDIC_037_TO_ASCII, decode_ebcdic_1142, encode_ebcdic_1142, encode_ebcdic_1142_char, translate_bytes,
    translate_bytes_inplace,
};

#[inline(never)]
pub fn translate_bytes_ascii_to_037(output: &mut [u8], input: &[u8]) -> Result<(), Error> {
    translate_bytes(output, input, &ASCII_TO_EBCDIC_037)
}

#[inline(never)]
pub fn translate_bytes_037_to_ascii(output: &mut [u8], input: &[u8]) -> Result<(), Error> {
    translate_bytes(output, input, &EBCDIC_037_TO_ASCII)
}

#[inline(never)]
pub fn translate_bytes_inplace_ascii_to_037(buf: &mut [u8]) {
    translate_bytes_inplace(buf, &ASCII_TO_EBCDIC_037)
}

#[inline(never)]
pub fn translate_bytes_inplace_037_to_ascii(buf: &mut [u8]) {
    translate_bytes_inplace(buf, &EBCDIC_037_TO_ASCII)
}

#[inline(never)]
pub fn encode_ebcdic_1142_char_lookup(ch: char) -> Option<u8> {
    encode_ebcdic_1142_char(ch)
}

#[inline(never)]
pub fn encode_ebcdic_1142_runtime<'a>(output: &mut &'a mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
    encode_ebcdic_1142(output, input)
}

#[inline(never)]
pub fn decode_ebcdic_1142_runtime<'a>(output: &mut &'a mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
    decode_ebcdic_1142(output, input)
}
