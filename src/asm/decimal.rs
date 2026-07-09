use crate::Error;
use crate::primitive::decimal::{
    MAX_DECIMAL_LEN, decode_decimal_ascii_fixed, decode_decimal_ebcdic_blankable_fixed, decode_decimal_ebcdic_fixed,
    decode_decimal_ebcdic_signed_fixed, decode_decimal_implied, decode_decimal_packed_fixed, decode_decimal_packed_signed_fixed,
    decode_negative_prefix, decode_overpunch_digit, decode_packed_sign, decode_sign, encode_decimal_ascii_fixed,
    encode_decimal_ebcdic_blankable_fixed, encode_decimal_ebcdic_fixed, encode_decimal_ebcdic_signed_fixed, encode_decimal_implied,
    encode_decimal_packed_fixed, encode_decimal_packed_signed_fixed, encode_negative_prefix, encode_overpunch_digit, encode_packed_sign,
    encode_sign, format_i64, format_u64, packed_decimal_max_digits, parse_i64, parse_u64, parse_usize, prepend_minus,
};

#[inline(never)]
pub fn format_u64_to_buf(output: &mut [u8; MAX_DECIMAL_LEN], value: u64) -> &[u8] {
    format_u64(output, value)
}

#[inline(never)]
pub fn format_i64_to_buf(output: &mut [u8; MAX_DECIMAL_LEN], value: i64) -> &[u8] {
    format_i64(output, value)
}

#[inline(never)]
pub fn parse_u64_from_bytes(bytes: &[u8]) -> Result<u64, Error> {
    parse_u64(bytes)
}

#[inline(never)]
pub fn parse_usize_from_bytes(bytes: &[u8]) -> Result<usize, Error> {
    parse_usize(bytes)
}

#[inline(never)]
pub fn parse_i64_from_bytes(bytes: &[u8]) -> Result<i64, Error> {
    parse_i64(bytes)
}

#[inline(never)]
pub fn decode_sign_cd(input: &mut &[u8]) -> Result<bool, Error> {
    decode_sign(input, b'C', b'D')
}

#[inline(never)]
pub fn encode_sign_cd(output: &mut &mut [u8], negative: bool) -> Result<(), Error> {
    encode_sign(output, negative, b'C', b'D')
}

#[inline(never)]
pub fn encode_negative_prefix_minus(output: &mut &mut [u8], negative: bool) -> Result<(), Error> {
    encode_negative_prefix(output, negative, b'-')
}

#[inline(never)]
pub fn decode_negative_prefix_minus(input: &mut &[u8]) -> Result<bool, Error> {
    decode_negative_prefix(input, b'-')
}

#[inline(never)]
pub fn prepend_minus_to_digits<'a>(output: &mut &'a mut [u8], digits: &[u8]) -> Result<&'a mut [u8], Error> {
    prepend_minus(output, digits)
}

#[inline(never)]
pub fn encode_decimal_implied_scale2_signed<'a>(output: &mut &'a mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
    encode_decimal_implied(output, input, 2, 12, true)
}

#[inline(never)]
pub fn decode_decimal_implied_scale2<'a>(output: &mut &'a mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
    decode_decimal_implied(output, input, 2)
}

#[inline(never)]
pub fn encode_overpunch_digit_for(negative: bool, digit: u8) -> u8 {
    encode_overpunch_digit(negative, digit)
}

#[inline(never)]
pub fn decode_overpunch_digit_byte(input: u8) -> Result<(bool, u8), Error> {
    decode_overpunch_digit(input)
}

#[inline(never)]
pub fn encode_packed_sign_signed(negative: bool) -> u8 {
    encode_packed_sign(negative, true)
}

#[inline(never)]
pub fn encode_packed_sign_unsigned(negative: bool) -> u8 {
    encode_packed_sign(negative, false)
}

#[inline(never)]
pub fn decode_packed_sign_nibble(input: u8) -> Result<bool, Error> {
    decode_packed_sign(input)
}

#[inline(never)]
pub fn packed_decimal_max_digits_8(bytes_len: usize) -> Result<usize, Error> {
    packed_decimal_max_digits(bytes_len)
}

#[inline(never)]
pub fn encode_decimal_ascii_fixed_2(output: &mut &mut [u8], value: usize) -> Result<(), Error> {
    encode_decimal_ascii_fixed(output, value, 2)
}

#[inline(never)]
pub fn encode_decimal_ebcdic_fixed_2(output: &mut &mut [u8], value: usize) -> Result<(), Error> {
    encode_decimal_ebcdic_fixed(output, value, 2)
}

#[inline(never)]
pub fn encode_decimal_ebcdic_blankable_fixed_2(output: &mut &mut [u8], value: usize) -> Result<(), Error> {
    encode_decimal_ebcdic_blankable_fixed(output, value, 2)
}

#[inline(never)]
pub fn decode_decimal_ascii_fixed_2(input: &mut &[u8]) -> Result<usize, Error> {
    decode_decimal_ascii_fixed(input, 2)
}

#[inline(never)]
pub fn decode_decimal_ebcdic_fixed_2(input: &mut &[u8]) -> Result<usize, Error> {
    decode_decimal_ebcdic_fixed(input, 2)
}

#[inline(never)]
pub fn decode_decimal_ebcdic_blankable_fixed_2(input: &mut &[u8]) -> Result<usize, Error> {
    decode_decimal_ebcdic_blankable_fixed(input, 2)
}

#[inline(never)]
pub fn encode_decimal_ebcdic_signed_fixed_8(output: &mut &mut [u8], input: &[u8]) -> Result<(), Error> {
    encode_decimal_ebcdic_signed_fixed(output, input, 8)
}

#[inline(never)]
pub fn decode_decimal_ebcdic_signed_fixed_8<'a>(input: &mut &[u8], output: &mut &'a mut [u8]) -> Result<&'a mut [u8], Error> {
    decode_decimal_ebcdic_signed_fixed(input, output, 8)
}

#[inline(never)]
pub fn encode_decimal_packed_fixed_8(output: &mut &mut [u8], input: &[u8]) -> Result<(), Error> {
    encode_decimal_packed_fixed(output, input, 8)
}

#[inline(never)]
pub fn encode_decimal_packed_signed_fixed_8(output: &mut &mut [u8], input: &[u8]) -> Result<(), Error> {
    encode_decimal_packed_signed_fixed(output, input, 8)
}

#[inline(never)]
pub fn decode_decimal_packed_fixed_8<'a>(input: &mut &[u8], output: &mut &'a mut [u8]) -> Result<&'a mut [u8], Error> {
    decode_decimal_packed_fixed(input, output, 8)
}

#[inline(never)]
pub fn decode_decimal_packed_signed_fixed_8<'a>(input: &mut &[u8], output: &mut &'a mut [u8]) -> Result<&'a mut [u8], Error> {
    decode_decimal_packed_signed_fixed(input, output, 8)
}
