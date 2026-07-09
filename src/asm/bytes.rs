use crate::Error;
use crate::primitive::bytes::{
    all_bytes_eq, contains_byte, copy_bytes, decode_exact_bytes, decode_filled_prefix, encode_exact_bytes, fill_repeated_block, fill_tail,
    reserve_filled_area, split_delimited_bytes, validate_all_bytes, validate_exact_length, validate_repeating_block,
};

#[inline(never)]
pub fn validate_exact_length_8(input: &[u8]) -> Result<(), Error> {
    validate_exact_length(input, 8)
}

#[inline(never)]
pub fn copy_bytes_through<'a>(output: &mut &'a mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
    copy_bytes(output, input)
}

#[inline(never)]
pub fn encode_exact_bytes_8(output: &mut &mut [u8], input: &[u8]) -> Result<(), Error> {
    encode_exact_bytes(output, input, 8)
}

#[inline(never)]
pub fn decode_exact_bytes_8<'a>(input: &mut &'a [u8]) -> Result<&'a [u8], Error> {
    decode_exact_bytes(input, 8)
}

#[inline(never)]
pub fn reserve_filled_area_8_ebcdic_space<'a>(output: &mut &'a mut [u8]) -> Result<&'a mut [u8], Error> {
    reserve_filled_area(output, 8, 0x40)
}

#[inline(never)]
pub fn decode_filled_prefix_8_ebcdic_space<'a>(input: &mut &'a [u8], used_len: usize) -> Result<&'a [u8], Error> {
    decode_filled_prefix(input, 8, used_len, 0x40)
}

#[inline(never)]
pub fn fill_tail_ebcdic_space(output: &mut [u8], used_len: usize) -> Result<(), Error> {
    fill_tail(output, used_len, 0x40)
}

#[inline(never)]
pub fn all_bytes_eq_ebcdic_space(input: &[u8]) -> bool {
    all_bytes_eq(input, 0x40)
}

#[inline(never)]
pub fn validate_all_bytes_ebcdic_space(input: &[u8]) -> Result<(), Error> {
    validate_all_bytes(input, 0x40)
}

#[inline(never)]
pub fn fill_repeated_block_runtime(output: &mut [u8], used_len: usize, block: &[u8]) -> Result<(), Error> {
    fill_repeated_block(output, used_len, block)
}

#[inline(never)]
pub fn validate_repeating_block_runtime(input: &[u8], block: &[u8]) -> Result<(), Error> {
    validate_repeating_block(input, block)
}

#[inline(never)]
pub fn contains_byte_pipe(input: &[u8]) -> bool {
    contains_byte(input, b'|')
}

#[inline(never)]
pub fn split_delimited_bytes_pipe<'a>(input: &mut &'a [u8]) -> Result<&'a [u8], Error> {
    split_delimited_bytes(input, b'|', true)
}
