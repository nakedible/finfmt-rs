use crate::Error;
use crate::primitive::bytes::{
    all_bytes_eq, contains_byte, copy_bytes, decode_padded_bytes, encode_exact_bytes, fill_repeated_block, fill_tail, reserve_bytes,
    reserve_filled_area, split_delimited_bytes, take_bytes, validate_exact_length, validate_repeated_block,
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
pub fn take_bytes_8<'a>(input: &mut &'a [u8]) -> Result<&'a [u8], Error> {
    take_bytes(input, 8)
}

#[inline(never)]
pub fn reserve_bytes_8<'a>(output: &mut &'a mut [u8]) -> Result<&'a mut [u8], Error> {
    reserve_bytes(output, 8)
}

#[inline(never)]
pub fn reserve_filled_area_8_ebcdic_space<'a>(output: &mut &'a mut [u8]) -> Result<&'a mut [u8], Error> {
    reserve_filled_area(output, 8, 0x40)
}

#[inline(never)]
pub fn decode_padded_bytes_8_ebcdic_space<'a>(input: &mut &'a [u8], used_len: usize) -> Result<&'a [u8], Error> {
    decode_padded_bytes(input, 8, used_len, 0x40)
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
pub fn fill_repeated_block_runtime(output: &mut [u8], used_len: usize, block: &[u8]) -> Result<(), Error> {
    fill_repeated_block(output, used_len, block)
}

#[inline(never)]
pub fn validate_repeated_block_runtime(input: &[u8], block: &[u8]) -> Result<(), Error> {
    validate_repeated_block(input, block)
}

#[inline(never)]
pub fn contains_byte_pipe(input: &[u8]) -> bool {
    contains_byte(input, b'|')
}

#[inline(never)]
pub fn split_delimited_bytes_pipe<'a>(input: &mut &'a [u8]) -> (&'a [u8], bool) {
    split_delimited_bytes(input, b'|')
}

#[inline(never)]
pub fn fill_repeated_block_4(output: &mut [u8], block: &[u8; 4]) -> Result<(), Error> {
    fill_repeated_block(output, 0, block)
}

#[inline(never)]
pub fn validate_repeated_block_4(input: &[u8], block: &[u8; 4]) -> Result<(), Error> {
    validate_repeated_block(input, block)
}
