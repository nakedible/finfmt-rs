#[cfg(all(not(debug_assertions), feature = "no-panic"))]
use no_panic::no_panic;

use crate::Error;
use crate::utils::cold_path;

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_exact_length(input: &[u8], len: usize) -> Result<(), Error> {
    if input.len() != len {
        cold_path();
        return Err(Error::Invalid);
    }
    Ok(())
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn copy_bytes<'a>(output: &mut &'a mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
    let buf = output.split_off_mut(..input.len()).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    buf.copy_from_slice(input);
    Ok(buf)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn encode_exact_bytes(output: &mut &mut [u8], input: &[u8], len: usize) -> Result<(), Error> {
    validate_exact_length(input, len)?;
    let _ = copy_bytes(output, input)?;
    Ok(())
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn decode_exact_bytes<'a>(input: &mut &'a [u8], len: usize) -> Result<&'a [u8], Error> {
    input.split_off(..len).ok_or_else(|| {
        cold_path();
        Error::UnexpectedEof
    })
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn reserve_filled_area<'a>(output: &mut &'a mut [u8], len: usize, fill: u8) -> Result<&'a mut [u8], Error> {
    let area = output.split_off_mut(..len).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    area.fill(fill);
    Ok(area)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn decode_filled_prefix<'a>(input: &mut &'a [u8], total_len: usize, used_len: usize, fill: u8) -> Result<&'a [u8], Error> {
    if used_len > total_len {
        cold_path();
        return Err(Error::Invalid);
    }
    let area = input.split_off(..total_len).ok_or_else(|| {
        cold_path();
        Error::UnexpectedEof
    })?;
    if area[used_len..].iter().any(|&byte| byte != fill) {
        cold_path();
        return Err(Error::Invalid);
    }
    Ok(&area[..used_len])
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn fill_tail(output: &mut [u8], used_len: usize, fill: u8) -> Result<(), Error> {
    let tail = output.get_mut(used_len..).ok_or_else(|| {
        cold_path();
        Error::Invalid
    })?;
    tail.fill(fill);
    Ok(())
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn all_bytes_eq(input: &[u8], fill: u8) -> bool {
    input.iter().all(|&byte| byte == fill)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_all_bytes(input: &[u8], fill: u8) -> Result<(), Error> {
    if !all_bytes_eq(input, fill) {
        cold_path();
        return Err(Error::Invalid);
    }
    Ok(())
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn fill_repeated_block(output: &mut [u8], used_len: usize, block: &[u8]) -> Result<(), Error> {
    if block.is_empty() {
        cold_path();
        return Err(Error::Internal);
    }
    let tail = output.get_mut(used_len..).ok_or_else(|| {
        cold_path();
        Error::Invalid
    })?;
    if !tail.len().is_multiple_of(block.len()) {
        cold_path();
        return Err(Error::Internal);
    }
    let mut offset = 0usize;
    while offset < tail.len() {
        tail[offset..offset + block.len()].copy_from_slice(block);
        offset += block.len();
    }
    Ok(())
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_repeating_block(input: &[u8], block: &[u8]) -> Result<(), Error> {
    if block.is_empty() {
        cold_path();
        return Err(Error::Internal);
    }
    if !input.len().is_multiple_of(block.len()) {
        cold_path();
        return Err(Error::Invalid);
    }
    let mut offset = 0usize;
    while offset < input.len() {
        if input[offset..offset + block.len()] != *block {
            cold_path();
            return Err(Error::Invalid);
        }
        offset += block.len();
    }
    Ok(())
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn contains_byte(input: &[u8], byte: u8) -> bool {
    input.contains(&byte)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn split_delimited_bytes<'a>(input: &mut &'a [u8], separator: u8, expect_separator: bool) -> Result<&'a [u8], Error> {
    if !expect_separator {
        let segment = *input;
        *input = &[];
        return Ok(segment);
    }
    let split_at = input.iter().position(|&byte| byte == separator).ok_or_else(|| {
        cold_path();
        Error::Invalid
    })?;
    let segment = input.split_off(..split_at).ok_or_else(|| {
        cold_path();
        Error::Invalid
    })?;
    *input = input.split_off(1..).ok_or_else(|| {
        cold_path();
        Error::Invalid
    })?;
    Ok(segment)
}

#[cfg(test)]
mod tests {
    use super::{
        all_bytes_eq, contains_byte, copy_bytes, decode_exact_bytes, decode_filled_prefix, encode_exact_bytes, fill_repeated_block,
        fill_tail, reserve_filled_area, split_delimited_bytes, validate_all_bytes, validate_exact_length, validate_repeating_block,
    };
    use crate::Error;

    fn encode<const OUT: usize>(input: &[u8], len: usize) -> Result<[u8; OUT], Error> {
        let mut output = [0u8; OUT];
        let mut out_ptr = output.as_mut_slice();
        encode_exact_bytes(&mut out_ptr, input, len)?;
        Ok(output)
    }

    fn decode(input: &[u8], len: usize) -> Result<Vec<u8>, Error> {
        let mut input = input;
        Ok(decode_exact_bytes(&mut input, len)?.to_vec())
    }

    fn copy<const N: usize>(input: &[u8]) -> Result<[u8; N], Error> {
        let mut output = [0u8; N];
        let mut out_ptr = output.as_mut_slice();
        let _ = copy_bytes(&mut out_ptr, input)?;
        Ok(output)
    }

    fn reserve_filled<const N: usize>(fill: u8) -> Result<[u8; N], Error> {
        let mut output = [0u8; N];
        let mut out_ptr = output.as_mut_slice();
        let _ = reserve_filled_area(&mut out_ptr, N, fill)?;
        Ok(output)
    }

    fn decode_prefix(input: &[u8], total_len: usize, used_len: usize, fill: u8) -> Result<Vec<u8>, Error> {
        let mut input = input;
        Ok(decode_filled_prefix(&mut input, total_len, used_len, fill)?.to_vec())
    }

    fn fill_tail_buf<const N: usize>(used_len: usize, fill: u8) -> Result<[u8; N], Error> {
        let mut output = [0u8; N];
        fill_tail(&mut output, used_len, fill)?;
        Ok(output)
    }

    fn fill_repeated_block_buf<const N: usize>(used_len: usize, block: &[u8]) -> Result<[u8; N], Error> {
        let mut output = [0u8; N];
        fill_repeated_block(&mut output, used_len, block)?;
        Ok(output)
    }

    fn split_delimited(input: &[u8], separator: u8, expect_separator: bool) -> Result<(Vec<u8>, Vec<u8>), Error> {
        let mut input = input;
        let segment = split_delimited_bytes(&mut input, separator, expect_separator)?;
        Ok((segment.to_vec(), input.to_vec()))
    }

    #[test]
    fn test_fixed_bytes_helpers() {
        assert_eq!(validate_exact_length(b"\x12\x34", 2), Ok(()));
        assert_eq!(validate_exact_length(b"\x12", 2), Err(Error::Invalid));
        assert_eq!(copy::<2>(b"\x12\x34"), Ok([0x12, 0x34]));
        assert_eq!(copy::<1>(b"\x12\x34"), Err(Error::BufferOverflow));
        assert_eq!(encode::<2>(b"\x12\x34", 2), Ok([0x12, 0x34]));
        assert_eq!(encode::<1>(b"\x12\x34", 2), Err(Error::BufferOverflow));
        assert_eq!(encode::<2>(b"\x12", 2), Err(Error::Invalid));
        assert_eq!(decode(b"\x12\x34", 2), Ok(vec![0x12, 0x34]));
        assert_eq!(decode(b"\x12", 2), Err(Error::UnexpectedEof));
        assert_eq!(reserve_filled::<3>(0x40), Ok([0x40, 0x40, 0x40]));
        assert_eq!(decode_prefix(b"\x12\x34\x40\x40", 4, 2, 0x40), Ok(vec![0x12, 0x34]));
        assert_eq!(decode_prefix(b"\x12\x34\x40\x41", 4, 2, 0x40), Err(Error::Invalid));
        assert_eq!(decode_prefix(b"\x12\x34", 4, 2, 0x40), Err(Error::UnexpectedEof));
        assert_eq!(decode_prefix(b"\x12\x34", 2, 3, 0x40), Err(Error::Invalid));
        assert_eq!(fill_tail_buf::<4>(2, 0x40), Ok([0x00, 0x00, 0x40, 0x40]));
        assert_eq!(fill_tail_buf::<2>(3, 0x40), Err(Error::Invalid));
        assert!(all_bytes_eq(b"\x40\x40", 0x40));
        assert!(!all_bytes_eq(b"\x40\x41", 0x40));
        assert!(contains_byte(b"\x12\x34", 0x34));
        assert!(!contains_byte(b"\x12\x34", 0x56));
        assert_eq!(validate_all_bytes(b"\x40\x40", 0x40), Ok(()));
        assert_eq!(validate_all_bytes(b"\x40\x41", 0x40), Err(Error::Invalid));
        assert_eq!(
            fill_repeated_block_buf::<6>(2, b"\x12\x34"),
            Ok([0x00, 0x00, 0x12, 0x34, 0x12, 0x34])
        );
        assert_eq!(fill_repeated_block_buf::<5>(2, b"\x12\x34"), Err(Error::Internal));
        assert_eq!(fill_repeated_block_buf::<5>(2, b""), Err(Error::Internal));
        assert_eq!(validate_repeating_block(b"\x12\x34\x12\x34", b"\x12\x34"), Ok(()));
        assert_eq!(validate_repeating_block(b"\x12\x34\x56\x78", b"\x12\x34"), Err(Error::Invalid));
        assert_eq!(validate_repeating_block(b"\x12", b"\x12\x34"), Err(Error::Invalid));
        assert_eq!(validate_repeating_block(b"\x12", b""), Err(Error::Internal));
        assert_eq!(split_delimited(b"A|B", b'|', true), Ok((b"A".to_vec(), b"B".to_vec())));
        assert_eq!(split_delimited(b"AB", b'|', false), Ok((b"AB".to_vec(), vec![])));
        assert_eq!(split_delimited(b"AB", b'|', true), Err(Error::Invalid));
    }
}
