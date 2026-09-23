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

/// Reserve the next `len` output bytes, advancing the cursor past them.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn reserve_bytes<'a>(output: &mut &'a mut [u8], len: usize) -> Result<&'a mut [u8], Error> {
    output.split_off_mut(..len).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })
}

/// Take the next `len` input bytes, advancing the cursor past them.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn take_bytes<'a>(input: &mut &'a [u8], len: usize) -> Result<&'a [u8], Error> {
    input.split_off(..len).ok_or_else(|| {
        cold_path();
        Error::UnexpectedEof
    })
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn copy_bytes<'a>(output: &mut &'a mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
    let buf = reserve_bytes(output, input.len())?;
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
pub fn reserve_filled_area<'a>(output: &mut &'a mut [u8], len: usize, fill: u8) -> Result<&'a mut [u8], Error> {
    let area = output.split_off_mut(..len).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    area.fill(fill);
    Ok(area)
}

/// Consume `total_len` bytes, check that the suffix after `used_len` contains only
/// `fill`, and return the used prefix. The supplied `used_len` determines the
/// boundary; fill bytes within the used prefix are preserved.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn decode_padded_bytes<'a>(input: &mut &'a [u8], total_len: usize, used_len: usize, fill: u8) -> Result<&'a [u8], Error> {
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
    for chunk in tail.chunks_exact_mut(block.len()) {
        chunk.copy_from_slice(block);
    }
    Ok(())
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_repeated_block(input: &[u8], block: &[u8]) -> Result<(), Error> {
    if block.is_empty() {
        cold_path();
        return Err(Error::Internal);
    }
    if !input.len().is_multiple_of(block.len()) {
        cold_path();
        return Err(Error::Invalid);
    }
    for chunk in input.chunks_exact(block.len()) {
        if chunk != block {
            cold_path();
            return Err(Error::Invalid);
        }
    }
    Ok(())
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn contains_byte(input: &[u8], byte: u8) -> bool {
    input.contains(&byte)
}

/// Consume through the first separator, returning the preceding bytes and whether
/// a separator was found. If absent, consume the remainder and return it with `false`.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn split_delimited_bytes<'a>(input: &mut &'a [u8], separator: u8) -> (&'a [u8], bool) {
    let mut parts = input.splitn(2, |&byte| byte == separator);
    if let (Some(segment), Some(rest)) = (parts.next(), parts.next()) {
        *input = rest;
        (segment, true)
    } else {
        (core::mem::take(input), false)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        all_bytes_eq, contains_byte, copy_bytes, decode_padded_bytes, encode_exact_bytes, fill_repeated_block, fill_tail,
        reserve_filled_area, split_delimited_bytes, take_bytes, validate_exact_length, validate_repeated_block,
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
        Ok(take_bytes(&mut input, len)?.to_vec())
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
        Ok(decode_padded_bytes(&mut input, total_len, used_len, fill)?.to_vec())
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
        assert_eq!(decode_prefix(b"A   ", 4, 2, b' '), Ok(b"A ".to_vec()));
        assert_eq!(decode_prefix(b"\x12\x34\x40\x41", 4, 2, 0x40), Err(Error::Invalid));
        assert_eq!(decode_prefix(b"\x12\x34", 4, 2, 0x40), Err(Error::UnexpectedEof));
        assert_eq!(decode_prefix(b"\x12\x34", 2, 3, 0x40), Err(Error::Invalid));
        assert_eq!(fill_tail_buf::<4>(2, 0x40), Ok([0x00, 0x00, 0x40, 0x40]));
        assert_eq!(fill_tail_buf::<2>(3, 0x40), Err(Error::Invalid));
        assert!(all_bytes_eq(b"\x40\x40", 0x40));
        assert!(!all_bytes_eq(b"\x40\x41", 0x40));
        assert!(contains_byte(b"\x12\x34", 0x34));
        assert!(!contains_byte(b"\x12\x34", 0x56));
        assert_eq!(
            fill_repeated_block_buf::<6>(2, b"\x12\x34"),
            Ok([0x00, 0x00, 0x12, 0x34, 0x12, 0x34])
        );
        assert_eq!(fill_repeated_block_buf::<5>(2, b"\x12\x34"), Err(Error::Internal));
        assert_eq!(fill_repeated_block_buf::<5>(2, b""), Err(Error::Internal));
        assert_eq!(validate_repeated_block(b"\x12\x34\x12\x34", b"\x12\x34"), Ok(()));
        assert_eq!(validate_repeated_block(b"\x12\x34\x56\x78", b"\x12\x34"), Err(Error::Invalid));
        assert_eq!(validate_repeated_block(b"\x12", b"\x12\x34"), Err(Error::Invalid));
        assert_eq!(validate_repeated_block(b"\x12", b""), Err(Error::Internal));
    }

    #[test]
    fn test_repeated_block_boundaries() {
        assert_eq!(fill_repeated_block(&mut [], 0, b""), Err(Error::Internal));
        assert_eq!(fill_repeated_block(&mut [], 0, b"AB"), Ok(()));
        assert_eq!(validate_repeated_block(b"", b""), Err(Error::Internal));
        assert_eq!(validate_repeated_block(b"", b"AB"), Ok(()));
        let mut output = [0x55; 5];
        for (used, block, expected) in [
            (2, b"AB".as_slice(), Err(Error::Internal)),
            (usize::MAX, b"AB", Err(Error::Invalid)),
            (usize::MAX, b"", Err(Error::Internal)),
            (5, b"AB", Ok(())),
        ] {
            assert_eq!(fill_repeated_block(&mut output, used, block), expected);
            assert_eq!(output, [0x55; 5]);
        }
        assert_eq!(fill_repeated_block(&mut output, 1, b"AB"), Ok(()));
        assert_eq!(&output, b"UABAB");
        assert_eq!(fill_repeated_block(&mut output, 3, &[0]), Ok(()));
        assert_eq!(&output, b"UAB\0\0");
        for input in [b"XBAB", b"AXAB", b"ABXB", b"ABAX"] {
            assert_eq!(validate_repeated_block(input, b"AB"), Err(Error::Invalid));
        }
    }

    #[test]
    fn test_delimited_byte_boundaries() {
        for (wire, separator, segment, rest, terminated) in [
            (b"".as_slice(), b'|', b"".as_slice(), b"".as_slice(), false),
            (b"AB", b'|', b"AB", b"", false),
            (b"A|B", b'|', b"A", b"B", true),
            (b"|B", b'|', b"", b"B", true),
            (b"A|", b'|', b"A", b"", true),
            (b"|", b'|', b"", b"", true),
            (b"A||B", b'|', b"A", b"|B", true),
            (b"\0A\0B", 0, b"", b"A\0B", true),
        ] {
            let mut input = wire;
            assert_eq!(split_delimited_bytes(&mut input, separator), (segment, terminated));
            assert_eq!(input, rest);
        }
    }
}

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    use super::{fill_repeated_block, split_delimited_bytes, validate_repeated_block};
    use crate::Error;

    proptest! {
        #[test]
        fn repeated_blocks_preserve_prefix_and_match_reference(
            block in prop::collection::vec(any::<u8>(), 1..17),
            prefix in prop::collection::vec(any::<u8>(), 0..32),
            count in 0usize..32,
            corrupt in any::<usize>(),
        ) {
            let mut output = prefix.clone();
            output.resize(prefix.len() + block.len() * count, 0x55);
            prop_assert_eq!(fill_repeated_block(&mut output, prefix.len(), &block), Ok(()));
            prop_assert_eq!(&output[..prefix.len()], prefix.as_slice());
            let tail = &mut output[prefix.len()..];
            prop_assert_eq!(&*tail, block.repeat(count));
            prop_assert_eq!(validate_repeated_block(tail, &block), Ok(()));
            if !tail.is_empty() {
                let index = corrupt % tail.len();
                tail[index] ^= 1;
                prop_assert_eq!(validate_repeated_block(tail, &block), Err(Error::Invalid));
            }
        }

        #[test]
        fn delimited_bytes_match_first_separator(bytes in prop::collection::vec(any::<u8>(), 0..128), separator in any::<u8>()) {
            let mut input = bytes.as_slice();
            let (segment, terminated) = split_delimited_bytes(&mut input, separator);
            let boundary = bytes.iter().position(|&byte| byte == separator);
            let length = boundary.unwrap_or(bytes.len());
            prop_assert_eq!(segment, &bytes[..length]);
            prop_assert_eq!(terminated, boundary.is_some());
            prop_assert_eq!(input, &bytes[length + usize::from(terminated)..]);
        }
    }
}
