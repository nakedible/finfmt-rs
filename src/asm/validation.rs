use crate::Error;
use crate::primitive::validation::{
    parse_signed_decimal, split_signed_input, validate_alpha, validate_alphanum, validate_ascii, validate_ascii_printable,
    validate_bcd_bytes, validate_bcdz, validate_binary, validate_decimal_implied, validate_ebcdic_1142_text, validate_ebcdic_printable,
    validate_hex, validate_hex_even, validate_hex_lower, validate_hex_lower_even, validate_hex_upper, validate_hex_upper_even,
    validate_iso8859_1_str, validate_numeric, validate_track2, validate_upper_alpha, validate_upper_alphanum,
    validate_upper_ascii_printable,
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
pub fn validate_hex_upper_1_99(input: &[u8]) -> Result<usize, Error> {
    validate_hex_upper(input, 1, 99)
}

#[inline(never)]
pub fn validate_hex_lower_1_99(input: &[u8]) -> Result<usize, Error> {
    validate_hex_lower(input, 1, 99)
}

#[inline(never)]
pub fn validate_hex_even_2_98(input: &[u8]) -> Result<usize, Error> {
    validate_hex_even(input, 2, 98)
}

#[inline(never)]
pub fn validate_hex_upper_even_2_98(input: &[u8]) -> Result<usize, Error> {
    validate_hex_upper_even(input, 2, 98)
}

#[inline(never)]
pub fn validate_hex_lower_even_2_98(input: &[u8]) -> Result<usize, Error> {
    validate_hex_lower_even(input, 2, 98)
}

#[inline(never)]
pub fn validate_bcdz_1_99(input: &[u8]) -> Result<usize, Error> {
    validate_bcdz(input, 1, 99)
}

#[inline(never)]
pub fn validate_track2_1_37(input: &[u8]) -> Result<usize, Error> {
    validate_track2(input, 1, 37)
}

#[inline(never)]
pub fn validate_bcd_bytes_1_10(input: &[u8]) -> Result<usize, Error> {
    validate_bcd_bytes(input, 1, 10)
}

#[inline(never)]
pub fn validate_binary_1_99(input: &[u8]) -> Result<usize, Error> {
    validate_binary(input, 1, 99)
}

#[inline(never)]
pub fn validate_iso8859_1_str_1_99(input: &str) -> Result<usize, Error> {
    validate_iso8859_1_str(input, 1, 99)
}

#[inline(never)]
pub fn validate_ebcdic_1142_text_1_99(input: &[u8]) -> Result<usize, Error> {
    validate_ebcdic_1142_text(input, 1, 99)
}

#[inline(never)]
pub fn validate_ebcdic_printable_1_99(input: &[u8]) -> Result<usize, Error> {
    validate_ebcdic_printable(input, 1, 99)
}

#[inline(never)]
pub fn split_signed_input_runtime(input: &[u8]) -> Result<(bool, &[u8]), Error> {
    split_signed_input(input)
}

#[inline(never)]
pub fn parse_signed_decimal_19(input: &[u8]) -> Result<(bool, &[u8]), Error> {
    parse_signed_decimal(input, 19)
}

#[inline(never)]
pub fn validate_decimal_implied_scale2_signed(input: &[u8]) -> Result<usize, Error> {
    validate_decimal_implied(input, 2, 12, true)
}
