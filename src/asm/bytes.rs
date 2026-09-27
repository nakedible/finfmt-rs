use crate::Error;
use crate::primitive::bytes::{copy_bytes, fill_repeated, is_filled, reserve_bytes, reserve_filled, take_bytes, take_delimited};

#[inline(never)]
pub fn copy_bytes_through<'a>(output: &mut &'a mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
    copy_bytes(output, input)
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
pub fn reserve_filled_8_ebcdic_space<'a>(output: &mut &'a mut [u8]) -> Result<&'a mut [u8], Error> {
    reserve_filled(output, 8, 0x40)
}

#[inline(never)]
pub fn is_filled_ebcdic_space(input: &[u8]) -> bool {
    is_filled(input, 0x40)
}

#[inline(never)]
pub fn fill_repeated_runtime(output: &mut [u8], block: &[u8]) -> Result<(), Error> {
    fill_repeated(output, block)
}

#[inline(never)]
pub fn fill_repeated_4(output: &mut [u8], block: &[u8; 4]) -> Result<(), Error> {
    fill_repeated(output, block)
}

#[inline(never)]
pub fn take_delimited_pipe<'a>(input: &mut &'a [u8]) -> (&'a [u8], bool) {
    take_delimited(input, b'|')
}
