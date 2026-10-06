use crate::primitive::codepage::{
    decode_ascii_subset, decode_text, encode_ascii_subset, encode_char, encode_text, translate_bytes, translate_bytes_inplace,
};
use crate::{Charset, CharsetText, Cp037, Cp850, Cp1142, Error, Field, Latin1, Rest, ScalarFmt};

#[inline(never)]
pub fn translate_bytes_table(output: &mut [u8], input: &[u8], table: &[u8; 256]) -> Result<(), Error> {
    translate_bytes(output, input, table)
}

#[inline(never)]
pub fn translate_bytes_inplace_table(buf: &mut [u8], table: &[u8; 256]) {
    translate_bytes_inplace(buf, table)
}

#[inline(never)]
pub fn encode_ascii_subset_037(buf: &mut [u8]) {
    encode_ascii_subset::<Cp037>(buf)
}

#[inline(never)]
pub fn decode_ascii_subset_037(output: &mut [u8], input: &[u8]) -> Result<(), Error> {
    decode_ascii_subset::<Cp037>(output, input)
}

#[inline(never)]
pub fn encode_char_1142(ch: char) -> Option<u8> {
    encode_char::<Cp1142>(ch)
}

#[inline(never)]
pub fn encode_char_850(ch: char) -> Option<u8> {
    encode_char::<Cp850>(ch)
}

#[inline(never)]
pub fn encode_text_1142<'a>(output: &mut &'a mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
    encode_text::<Cp1142>(output, input)
}

#[inline(never)]
pub fn decode_text_1142<'a>(output: &mut &'a mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
    decode_text::<Cp1142>(output, input)
}

#[inline(never)]
pub fn encode_text_latin1<'a>(output: &mut &'a mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
    encode_text::<Latin1>(output, input)
}

#[inline(never)]
pub fn decode_text_latin1<'a>(output: &mut &'a mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
    decode_text::<Latin1>(output, input)
}

#[inline(never)]
pub fn encode_charset_1142_field(output: &mut &mut [u8], input: &[u8]) -> Result<(), Error> {
    Field::<CharsetText<Cp1142, 0, 99>, Rest, Charset<Cp1142>>::encode(output, &mut [][..], input)
}
