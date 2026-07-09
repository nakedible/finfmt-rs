use crate::Error;
use crate::primitive::int::{
    decode_binary_i64_be_fixed, decode_binary_u64_be_fixed, decode_nibble_int_fixed, decode_signed_magnitude_i64, encode_be_bytes,
    encode_binary_i64_be_fixed, encode_binary_u64_be_fixed, encode_nibble_int_fixed, extend_be_bytes, validate_binary_i64_be_fixed,
    validate_nibble_int_fixed,
};
use crate::primitive::nibble::HexUpper;

#[inline(never)]
pub fn encode_be_bytes_4_zero(output: &mut &mut [u8], bytes: &[u8]) -> Result<(), Error> {
    encode_be_bytes(output, bytes, 4, 0x00)
}

#[inline(never)]
pub fn extend_be_bytes_4_zero_to_8(input: &mut &[u8]) -> Result<[u8; 8], Error> {
    extend_be_bytes::<8>(input, 4, 0x00)
}

#[inline(never)]
pub fn validate_nibble_int_fixed_hex_upper_4(input: &[u8]) -> Result<(), Error> {
    validate_nibble_int_fixed::<HexUpper>(input, 4)
}

#[inline(never)]
pub fn encode_nibble_int_fixed_hex_upper_4(output: &mut &mut [u8], value: u64) -> Result<(), Error> {
    encode_nibble_int_fixed::<HexUpper>(output, value, 4)
}

#[inline(never)]
pub fn decode_nibble_int_fixed_hex_upper_4(input: &mut &[u8]) -> Result<u64, Error> {
    decode_nibble_int_fixed::<HexUpper>(input, 4)
}

#[inline(never)]
pub fn encode_binary_u64_be_fixed_4(output: &mut &mut [u8], value: u64) -> Result<(), Error> {
    encode_binary_u64_be_fixed(output, value, 4)
}

#[inline(never)]
pub fn decode_binary_u64_be_fixed_4(input: &mut &[u8]) -> Result<u64, Error> {
    decode_binary_u64_be_fixed(input, 4)
}

#[inline(never)]
pub fn validate_binary_i64_be_fixed_4(value: i64) -> Result<(), Error> {
    validate_binary_i64_be_fixed(value, 4)
}

#[inline(never)]
pub fn encode_binary_i64_be_fixed_4(output: &mut &mut [u8], value: i64) -> Result<(), Error> {
    encode_binary_i64_be_fixed(output, value, 4)
}

#[inline(never)]
pub fn decode_binary_i64_be_fixed_4(input: &mut &[u8]) -> Result<i64, Error> {
    decode_binary_i64_be_fixed(input, 4)
}

#[inline(never)]
pub fn decode_signed_magnitude_i64_combine(negative: bool, magnitude: u64) -> Result<i64, Error> {
    decode_signed_magnitude_i64(negative, magnitude)
}
