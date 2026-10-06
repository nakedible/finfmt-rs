#[cfg(all(not(debug_assertions), feature = "no-panic"))]
use no_panic::no_panic;

use crate::Error;
use crate::utils::cold_path;

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

/// Reserve the next `len` output bytes and fill them with `fill`.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn reserve_filled<'a>(output: &mut &'a mut [u8], len: usize, fill: u8) -> Result<&'a mut [u8], Error> {
    let area = reserve_bytes(output, len)?;
    area.fill(fill);
    Ok(area)
}

/// Whether every byte of `input` equals `fill`; true for empty input.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn is_filled(input: &[u8], fill: u8) -> bool {
    input.iter().all(|&byte| byte == fill)
}

/// Fill `output` with back-to-back copies of `block`. An empty block, or an
/// output that is not a whole number of blocks, is a format error.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn fill_repeated(output: &mut [u8], block: &[u8]) -> Result<(), Error> {
    if block.is_empty() || !output.len().is_multiple_of(block.len()) {
        cold_path();
        return Err(Error::Internal);
    }
    for chunk in output.chunks_exact_mut(block.len()) {
        chunk.copy_from_slice(block);
    }
    Ok(())
}

/// Take input through the first `separator`, returning the bytes before it and
/// whether it was found. Without a separator, take and return the remainder.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn take_delimited<'a>(input: &mut &'a [u8], separator: u8) -> (&'a [u8], bool) {
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
    use super::{copy_bytes, fill_repeated, is_filled, reserve_bytes, reserve_filled, take_bytes, take_delimited};
    use crate::Error;

    #[test]
    fn test_cursor_helpers() {
        let mut storage = [0xA5; 4];
        for (len, expected) in [(0, Ok(0)), (3, Ok(3)), (4, Ok(4)), (5, Err(Error::BufferOverflow))] {
            let mut out = storage.as_mut_slice();
            assert_eq!(reserve_bytes(&mut out, len).map(|area| area.len()), expected);
            assert_eq!(out.len(), expected.map_or(4, |used| 4 - used));
        }
        for (len, expected) in [(0, Ok(&b""[..])), (2, Ok(b"\x12\x34")), (3, Err(Error::UnexpectedEof))] {
            let mut input = &b"\x12\x34"[..];
            assert_eq!(take_bytes(&mut input, len), expected);
            assert_eq!(input.len(), expected.map_or(2, |taken| 2 - taken.len()));
        }
        let mut out = storage.as_mut_slice();
        assert_eq!(copy_bytes(&mut out, b"\x12\x34").map(|area| area.to_vec()), Ok(vec![0x12, 0x34]));
        assert_eq!(copy_bytes(&mut out, b"\x56\x78\x9A"), Err(Error::BufferOverflow));
        assert_eq!(reserve_filled(&mut out, 2, 0x40).map(|area| area.to_vec()), Ok(vec![0x40, 0x40]));
        assert!(out.is_empty());
        assert_eq!(storage, [0x12, 0x34, 0x40, 0x40]);
    }

    #[test]
    fn test_fill_helpers() {
        assert!(is_filled(b"", 0x40));
        assert!(is_filled(b"\x40\x40", 0x40));
        assert!(!is_filled(b"\x40\x41", 0x40));
    }

    #[test]
    fn test_repeated_fill() {
        let mut output = [0x55; 6];
        assert_eq!(fill_repeated(&mut output, b"\x12\x34"), Ok(()));
        assert_eq!(output, [0x12, 0x34, 0x12, 0x34, 0x12, 0x34]);
        assert_eq!(fill_repeated(&mut [], b"AB"), Ok(()));
        for block in [&b""[..], b"ABCD"] {
            let mut output = [0x55; 6];
            assert_eq!(fill_repeated(&mut output, block), Err(Error::Internal));
            assert_eq!(output, [0x55; 6]);
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
            assert_eq!(take_delimited(&mut input, separator), (segment, terminated));
            assert_eq!(input, rest);
        }
    }
}

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    use super::{fill_repeated, take_delimited};

    proptest! {
        #[test]
        fn repeated_fill_matches_reference(block in prop::collection::vec(any::<u8>(), 1..17), count in 0usize..32) {
            let mut output = vec![0x55; block.len() * count];
            prop_assert_eq!(fill_repeated(&mut output, &block), Ok(()));
            prop_assert_eq!(output, block.repeat(count));
        }

        #[test]
        fn delimited_bytes_match_first_separator(bytes in prop::collection::vec(any::<u8>(), 0..128), separator in any::<u8>()) {
            let mut input = bytes.as_slice();
            let (segment, terminated) = take_delimited(&mut input, separator);
            let boundary = bytes.iter().position(|&byte| byte == separator);
            let length = boundary.unwrap_or(bytes.len());
            prop_assert_eq!(segment, &bytes[..length]);
            prop_assert_eq!(terminated, boundary.is_some());
            prop_assert_eq!(input, &bytes[length + usize::from(terminated)..]);
        }
    }
}
