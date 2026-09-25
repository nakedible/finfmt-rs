use crate::Error;
use crate::primitive::validation::{
    validate_alpha, validate_alphanum, validate_ascii, validate_ascii_printable, validate_bcd_bytes, validate_bcdz, validate_byte_length,
    validate_bytes, validate_ebcdic_037_ascii, validate_ebcdic_1142_text, validate_ebcdic_printable, validate_hex, validate_hex_even,
    validate_iso8859_1_str, validate_lower_hex, validate_lower_hex_even, validate_numeric, validate_track2_chars, validate_upper_alpha,
    validate_upper_alphanum, validate_upper_ascii_printable, validate_upper_hex, validate_upper_hex_even,
};

#[inline(never)]
pub fn validate_numeric_1_19(input: &[u8]) -> Result<usize, Error> {
    validate_numeric(input, 1, 19)
}

#[inline(never)]
pub fn validate_alpha_1_99(input: &[u8]) -> Result<usize, Error> {
    validate_alpha(input, 1, 99)
}

#[inline(never)]
pub fn validate_alphanum_1_99(input: &[u8]) -> Result<usize, Error> {
    validate_alphanum(input, 1, 99)
}

#[inline(never)]
pub fn validate_ascii_1_99(input: &[u8]) -> Result<usize, Error> {
    validate_ascii(input, 1, 99)
}

#[inline(never)]
pub fn validate_ascii_printable_1_99(input: &[u8]) -> Result<usize, Error> {
    validate_ascii_printable(input, 1, 99)
}

#[inline(never)]
pub fn validate_upper_alpha_1_99(input: &[u8]) -> Result<usize, Error> {
    validate_upper_alpha(input, 1, 99)
}

#[inline(never)]
pub fn validate_upper_alphanum_1_99(input: &[u8]) -> Result<usize, Error> {
    validate_upper_alphanum(input, 1, 99)
}

#[inline(never)]
pub fn validate_upper_ascii_printable_1_99(input: &[u8]) -> Result<usize, Error> {
    validate_upper_ascii_printable(input, 1, 99)
}

#[inline(never)]
pub fn validate_hex_1_99(input: &[u8]) -> Result<usize, Error> {
    validate_hex(input, 1, 99)
}

#[inline(never)]
pub fn validate_upper_hex_1_99(input: &[u8]) -> Result<usize, Error> {
    validate_upper_hex(input, 1, 99)
}

#[inline(never)]
pub fn validate_lower_hex_1_99(input: &[u8]) -> Result<usize, Error> {
    validate_lower_hex(input, 1, 99)
}

#[inline(never)]
pub fn validate_hex_even_2_98(input: &[u8]) -> Result<usize, Error> {
    validate_hex_even(input, 2, 98)
}

#[inline(never)]
pub fn validate_upper_hex_even_2_98(input: &[u8]) -> Result<usize, Error> {
    validate_upper_hex_even(input, 2, 98)
}

#[inline(never)]
pub fn validate_lower_hex_even_2_98(input: &[u8]) -> Result<usize, Error> {
    validate_lower_hex_even(input, 2, 98)
}

#[inline(never)]
pub fn validate_bcdz_1_99(input: &[u8]) -> Result<usize, Error> {
    validate_bcdz(input, 1, 99)
}

#[inline(never)]
pub fn validate_track2_chars_1_37(input: &[u8]) -> Result<usize, Error> {
    validate_track2_chars(input, 1, 37)
}

#[inline(never)]
pub fn validate_bcd_bytes_1_10(input: &[u8]) -> Result<usize, Error> {
    validate_bcd_bytes(input, 1, 10)
}

#[inline(never)]
pub fn validate_byte_length_1_99(input: &[u8]) -> Result<usize, Error> {
    validate_byte_length(input, 1, 99)
}

#[inline(never)]
pub fn validate_iso8859_1_str_1_99(input: &str) -> Result<usize, Error> {
    validate_iso8859_1_str(input, 1, 99)
}

#[inline(never)]
pub fn validate_ebcdic_1142_text_1_99(input: &str) -> Result<usize, Error> {
    validate_ebcdic_1142_text(input, 1, 99)
}

#[inline(never)]
pub fn validate_ebcdic_037_ascii_1_99(input: &[u8]) -> Result<usize, Error> {
    validate_ebcdic_037_ascii(input, 1, 99)
}

#[inline(never)]
pub fn validate_ebcdic_printable_1_99(input: &[u8]) -> Result<usize, Error> {
    validate_ebcdic_printable(input, 1, 99)
}

#[inline(never)]
pub fn validate_bytes_track2_d_1_37(input: &[u8]) -> Result<usize, Error> {
    validate_bytes(input, 1, 37, |b| matches!(b, b'0'..=b'9' | b'=' | b'D'))
}
