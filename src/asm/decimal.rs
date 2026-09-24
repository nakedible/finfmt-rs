use crate::Error;
use crate::primitive::decimal::{
    MAX_INTEGER_TEXT_LEN, decode_ascii_decimal_fixed, decode_ebcdic_decimal_blank_zero_fixed, decode_ebcdic_decimal_fixed,
    decode_implied_decimal, decode_negative_prefix, decode_overpunch_digit, decode_packed_decimal_fixed,
    decode_packed_decimal_signed_fixed, decode_packed_sign, decode_sign, decode_zoned_decimal_signed_fixed, encode_ascii_decimal_fixed,
    encode_ebcdic_decimal_blank_zero_fixed, encode_ebcdic_decimal_fixed, encode_implied_decimal, encode_negative_prefix,
    encode_overpunch_digit, encode_packed_decimal_fixed, encode_packed_decimal_signed_fixed, encode_packed_sign, encode_sign,
    encode_zoned_decimal_signed_fixed, encoded_implied_decimal_len, format_i64, format_u64, packed_decimal_max_digits, parse_i64,
    parse_signed_decimal, parse_u64, parse_unsigned_decimal, parse_usize, prepend_minus, split_signed_input,
};

#[inline(never)]
pub fn format_u64_to_buf(output: &mut [u8; MAX_INTEGER_TEXT_LEN], value: u64) -> &[u8] {
    format_u64(output, value)
}

#[inline(never)]
pub fn format_i64_to_buf(output: &mut [u8; MAX_INTEGER_TEXT_LEN], value: i64) -> &[u8] {
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
pub fn decode_negative_prefix_minus(input: &mut &[u8]) -> bool {
    decode_negative_prefix(input, b'-')
}

#[inline(never)]
pub fn prepend_minus_to_digits<'a>(output: &mut &'a mut [u8], digits: &[u8]) -> Result<&'a mut [u8], Error> {
    prepend_minus(output, digits)
}

#[inline(never)]
pub fn encode_implied_decimal_scale2_signed<'a>(output: &mut &'a mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
    encode_implied_decimal(output, input, 2, 12, true)
}

#[inline(never)]
pub fn decode_implied_decimal_scale2<'a>(output: &mut &'a mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
    decode_implied_decimal(output, input, 2)
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
pub fn packed_decimal_max_digits_8(bytes_len: usize) -> usize {
    packed_decimal_max_digits(bytes_len)
}

#[inline(never)]
pub fn encode_ascii_decimal_fixed_2(output: &mut &mut [u8], value: usize) -> Result<(), Error> {
    encode_ascii_decimal_fixed(output, value, 2)
}

#[inline(never)]
pub fn encode_ascii_decimal_fixed_3(output: &mut &mut [u8], value: usize) -> Result<(), Error> {
    encode_ascii_decimal_fixed(output, value, 3)
}

#[inline(never)]
pub fn encode_ascii_ll_field(output: &mut &mut [u8], input: &[u8]) -> Result<(), Error> {
    use crate::{AsciiLength, Binary, Field, ScalarFmt};
    Field::<Binary<0, 99>, AsciiLength<2>>::encode(output, &mut [][..], input)
}

#[inline(never)]
pub fn encode_ascii_lll_field(output: &mut &mut [u8], input: &[u8]) -> Result<(), Error> {
    use crate::{AsciiLength, Binary, Field, ScalarFmt};
    Field::<Binary<0, 999>, AsciiLength<3>>::encode(output, &mut [][..], input)
}

#[inline(never)]
pub fn encode_ebcdic_decimal_fixed_2(output: &mut &mut [u8], value: usize) -> Result<(), Error> {
    encode_ebcdic_decimal_fixed(output, value, 2)
}

#[inline(never)]
pub fn encode_ebcdic_decimal_fixed_3(output: &mut &mut [u8], value: usize) -> Result<(), Error> {
    encode_ebcdic_decimal_fixed(output, value, 3)
}

#[inline(never)]
pub fn encode_ebcdic_decimal_blank_zero_fixed_2(output: &mut &mut [u8], value: usize) -> Result<(), Error> {
    encode_ebcdic_decimal_blank_zero_fixed(output, value, 2)
}

#[inline(never)]
pub fn decode_ascii_decimal_fixed_2(input: &mut &[u8]) -> Result<usize, Error> {
    decode_ascii_decimal_fixed(input, 2)
}

#[inline(never)]
pub fn decode_ebcdic_decimal_fixed_2(input: &mut &[u8]) -> Result<usize, Error> {
    decode_ebcdic_decimal_fixed(input, 2)
}

#[inline(never)]
pub fn decode_ebcdic_decimal_blank_zero_fixed_2(input: &mut &[u8]) -> Result<usize, Error> {
    decode_ebcdic_decimal_blank_zero_fixed(input, 2)
}

#[inline(never)]
pub fn encode_zoned_decimal_signed_fixed_8(output: &mut &mut [u8], input: &[u8]) -> Result<(), Error> {
    encode_zoned_decimal_signed_fixed(output, input, 8)
}

#[inline(never)]
pub fn decode_zoned_decimal_signed_fixed_8<'a>(input: &mut &[u8], output: &mut &'a mut [u8]) -> Result<&'a mut [u8], Error> {
    decode_zoned_decimal_signed_fixed(input, output, 8)
}

#[inline(never)]
pub fn encode_packed_decimal_fixed_8(output: &mut &mut [u8], input: &[u8]) -> Result<(), Error> {
    encode_packed_decimal_fixed(output, input, 8)
}

#[inline(never)]
pub fn encode_packed_decimal_signed_fixed_8(output: &mut &mut [u8], input: &[u8]) -> Result<(), Error> {
    encode_packed_decimal_signed_fixed(output, input, 8)
}

#[inline(never)]
pub fn decode_packed_decimal_fixed_8<'a>(input: &mut &[u8], output: &mut &'a mut [u8]) -> Result<&'a mut [u8], Error> {
    decode_packed_decimal_fixed(input, output, 8)
}

#[inline(never)]
pub fn decode_packed_decimal_signed_fixed_8<'a>(input: &mut &[u8], output: &mut &'a mut [u8]) -> Result<&'a mut [u8], Error> {
    decode_packed_decimal_signed_fixed(input, output, 8)
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
pub fn parse_unsigned_decimal_19(input: &[u8]) -> Result<&[u8], Error> {
    parse_unsigned_decimal(input, 19)
}

#[inline(never)]
pub fn encoded_implied_decimal_len_scale2_signed(input: &[u8]) -> Result<usize, Error> {
    encoded_implied_decimal_len(input, 2, 12, true)
}
