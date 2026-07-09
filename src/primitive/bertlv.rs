#[cfg(all(not(debug_assertions), feature = "no-panic"))]
use no_panic::no_panic;

use crate::Error;
use crate::primitive::bytes::copy_bytes;
use crate::primitive::nibble::{HexUpper, NibbleFormat, pack_expanded_nibbles, unpack_nibbles};
use crate::utils::{cold_path, take_scratch};

/// Encodes a BER tag by copying `input` into the `output` cursor.
///
/// This function does not validate tag correctness; it only copies bytes.
///
/// Parameters:
/// - `output`: Buffer cursor; advanced by the number of bytes written. The returned
///   slice is the portion of `output` that was written.
/// - `input`: Raw tag bytes to copy.
///
/// Returns:
/// - A mutable sub-slice of `output` that contains the written bytes.
///
/// Errors:
/// - `Error::BufferOverflow` if `output` does not have enough space.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn encode_bertag<'a>(output: &mut &'a mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
    copy_bytes(output, input)
}

/// Decodes a BER tag from `input`, advancing the slice to point after the tag.
///
/// Rules (high-tag-number form):
/// - One-octet tag if low 5 bits of the first octet are not `0x1F`.
/// - Otherwise, continuation octets are consumed until a byte with MSB 0 is found.
/// - Supports tags up to 4 octets; longer tags are rejected.
///
/// Returns:
/// - A sub-slice of `input` that contains the tag bytes.
///
/// Errors:
/// - `Error::Invalid` if the tag would exceed 4 octets.
/// - `Error::UnexpectedEof` if `input` does not contain enough bytes.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn decode_bertag<'a>(input: &mut &'a [u8]) -> Result<&'a [u8], Error> {
    match *input {
        [a, ..] if *a & 0x1F != 0x1F => input.split_off(..1).ok_or_else(|| {
            cold_path();
            Error::UnexpectedEof
        }),
        [_, 0x01..=0x7F, ..] => input.split_off(..2).ok_or_else(|| {
            cold_path();
            Error::UnexpectedEof
        }),
        [_, 0x81..=0xFF, c, ..] if *c < 0x80 => input.split_off(..3).ok_or_else(|| {
            cold_path();
            Error::UnexpectedEof
        }),
        [_, 0x81..=0xFF, _, d, ..] if *d < 0x80 => input.split_off(..4).ok_or_else(|| {
            cold_path();
            Error::UnexpectedEof
        }),
        [_, 0x00 | 0x80, ..] => {
            cold_path();
            Err(Error::Invalid)
        }
        [_, _, _, _, ..] => {
            cold_path();
            Err(Error::Invalid)
        }
        _ => {
            cold_path();
            Err(Error::UnexpectedEof)
        }
    }
}

/// Encodes a definite-form BER length into `output`.
///
/// Forms supported:
/// - Short form: `0..=0x7F` (single octet).
/// - Long form (1 octet of length): `0x80..=0xFF` (encoded as `0x81 NN`).
/// - Long form (2 octets of length): `0x0100..=0xFFFF` (encoded as `0x82 NN NN`).
///
/// Parameters:
/// - `output`: Buffer cursor; advanced by the number of bytes written.
/// - `input`: Length to encode (definite form).
///
/// Returns:
/// - A mutable sub-slice of `output` that contains the written bytes.
///
/// Errors:
/// - `Error::BufferOverflow` if `output` does not have enough space.
/// - `Error::Invalid` if `input` requires more than 2 length octets.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn encode_berlen<'a>(output: &mut &'a mut [u8], input: usize) -> Result<&'a mut [u8], Error> {
    match input {
        0..=0x7F => copy_bytes(output, &[input as u8]),
        0x80..=0xFF => copy_bytes(output, &[0x81, input as u8]),
        0x0100..=0xFFFF => copy_bytes(output, &[0x82, (input >> 8) as u8, input as u8]),
        _ => {
            cold_path();
            Err(Error::Invalid)
        }
    }
}

/// Decodes a definite-form BER length from `input`, advancing the slice.
///
/// Returns:
/// - The decoded length as `usize`.
///
/// Errors:
/// - `Error::Invalid` for indefinite form (`0x80`), reserved/unsupported forms
///   (`0x83..=0xFF`), or other invalid encodings.
/// - `Error::UnexpectedEof` if `input` does not contain enough bytes.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn decode_berlen(input: &mut &[u8]) -> Result<usize, Error> {
    match *input {
        [a @ 0x00..=0x7F, ..] => {
            input.split_off(..1).ok_or_else(|| {
                cold_path();
                Error::UnexpectedEof
            })?;
            Ok(*a as usize)
        }
        _ => decode_berlen_long(input),
    }
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn encoded_berlen(len: usize) -> Result<usize, Error> {
    match len {
        0..=0x7F => Ok(1),
        0x80..=0xFF => Ok(2),
        0x0100..=0xFFFF => Ok(3),
        _ => {
            cold_path();
            Err(Error::Invalid)
        }
    }
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn parse_hex_tag(tag: &str) -> Result<([u8; 4], usize), Error> {
    let bytes = tag.as_bytes();
    if bytes.is_empty() || bytes.len() > 8 || !bytes.len().is_multiple_of(2) {
        cold_path();
        return Err(Error::Internal);
    }
    let mut out = [0u8; 4];
    let mut packed = &mut out[..];
    pack_expanded_nibbles(&mut packed, bytes, &HexUpper::TABLE).map_err(|_| {
        cold_path();
        Error::Internal
    })?;
    Ok((out, bytes.len() / 2))
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn tag_eq_hex(tag_bytes: &[u8], tag_hex: &str) -> Result<bool, Error> {
    let (parsed, len) = parse_hex_tag(tag_hex)?;
    Ok(tag_bytes == &parsed[..len])
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub(crate) fn encode_hex_upper_into<'a>(scratch: &'a mut [u8], bytes: &[u8]) -> Result<&'a str, Error> {
    let needed = bytes.len().checked_mul(2).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    let out = scratch.get_mut(..needed).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    let mut unpacked = out;
    let out = unpack_nibbles(&mut unpacked, bytes, &HexUpper::DIGITS)?;
    core::str::from_utf8(out).map_err(|_| {
        cold_path();
        Error::Internal
    })
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub(crate) fn encode_unknown_tag_key_into<'a>(scratch: &'a mut [u8], tag: &[u8]) -> Result<&'a str, Error> {
    let hex_len = tag.len().checked_mul(2).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    let needed = (1 + hex_len).checked_add("_unknown".len()).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    let out = scratch.get_mut(..needed).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    out[0] = b't';
    let hex = out.get_mut(1..1 + hex_len).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    encode_hex_upper_into(hex, tag)?;
    out[1 + hex_len..].copy_from_slice(b"_unknown");
    core::str::from_utf8(out).map_err(|_| {
        cold_path();
        Error::Internal
    })
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub(crate) fn encode_hex_upper_scratch<'a>(scratch: &mut &'a mut [u8], bytes: &[u8]) -> Result<&'a str, Error> {
    let needed = bytes.len().checked_mul(2).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    let out = take_scratch(scratch, needed)?;
    encode_hex_upper_into(out, bytes)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub(crate) fn encode_unknown_tag_key_scratch<'a>(scratch: &mut &'a mut [u8], tag: &[u8]) -> Result<&'a str, Error> {
    let hex_len = tag.len().checked_mul(2).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    let needed = (1 + hex_len).checked_add("_unknown".len()).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    let out = take_scratch(scratch, needed)?;
    encode_unknown_tag_key_into(out, tag)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub(crate) fn parse_unknown_tag_key(key: &str) -> Result<([u8; 4], usize), Error> {
    let body = key
        .strip_prefix('t')
        .and_then(|rest| rest.strip_suffix("_unknown"))
        .ok_or_else(|| {
            cold_path();
            Error::Invalid
        })?;
    parse_hex_tag(body)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub(crate) fn encode_unknown_tlv_from_key(output: &mut &mut [u8], key: &str, value: &str) -> Result<(), Error> {
    let (tag_bytes, tag_len) = parse_unknown_tag_key(key)?;
    encode_unknown_tlv_from_tag(output, &tag_bytes[..tag_len], value)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub(crate) fn encode_unknown_tlv_from_tag(output: &mut &mut [u8], tag: &[u8], value: &str) -> Result<(), Error> {
    let used = value.len() / 2;
    let head_len = encoded_berlen(used)? + tag.len();
    let total = head_len + used;
    let out = output.split_off_mut(..total).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    let (head_buf, body_buf) = out.split_at_mut(head_len);
    let mut head = head_buf;
    encode_bertag(&mut head, tag)?;
    encode_berlen(&mut head, used)?;
    let mut body = body_buf;
    pack_expanded_nibbles(&mut body, value.as_bytes(), &<HexUpper as NibbleFormat>::TABLE)?;
    Ok(())
}

#[derive(Clone, Copy)]
pub struct BerTlvEntry<'a> {
    pub tag: &'a [u8],
    pub value: &'a [u8],
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn decode_ber_tlv_entry<'a>(input: &mut &'a [u8]) -> Result<Option<BerTlvEntry<'a>>, Error> {
    if input.is_empty() {
        return Ok(None);
    }
    let tag = decode_bertag(input)?;
    let len = decode_berlen(input)?;
    let value = input.split_off(..len).ok_or_else(|| {
        cold_path();
        Error::UnexpectedEof
    })?;
    Ok(Some(BerTlvEntry { tag, value }))
}

/// Cold path for decoding long-form BER lengths (1-2 length octets).
///
/// Supports:
/// - `0x81 NN` -> 1 length octet
/// - `0x82 NN NN` -> 2 length octets
///
/// Errors:
/// - `Error::Invalid` for `0x80` (indefinite) and `0x83..=0xFF`.
/// - `Error::UnexpectedEof` if `input` does not contain enough bytes.
#[cold]
#[inline(never)]
fn decode_berlen_long(input: &mut &[u8]) -> Result<usize, Error> {
    match *input {
        [0x81, b, ..] => {
            input.split_off(..2).ok_or_else(|| {
                cold_path();
                Error::UnexpectedEof
            })?;
            Ok(*b as usize)
        }
        [0x82, b, c, ..] => {
            input.split_off(..3).ok_or_else(|| {
                cold_path();
                Error::UnexpectedEof
            })?;
            Ok(u16::from_be_bytes([*b, *c]) as usize)
        }
        [0x80, ..] | [0x83..=0xFF, ..] => {
            cold_path();
            Err(Error::Invalid)
        }
        _ => {
            cold_path();
            Err(Error::UnexpectedEof)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enc_tag(input: &[u8], buf_len: usize) -> Result<Vec<u8>, Error> {
        let mut storage = [0u8; 8];
        let mut out = &mut storage[..buf_len];
        let initial_len = out.len();
        let res = encode_bertag(&mut out, input)?;
        assert_eq!(out.len(), initial_len - res.len(), "cursor advancement");
        Ok(res.to_vec())
    }

    fn dec_tag(input: &[u8]) -> Result<(Vec<u8>, Vec<u8>), Error> {
        let mut inp = input;
        let tag = decode_bertag(&mut inp)?;
        Ok((tag.to_vec(), inp.to_vec()))
    }

    fn enc_len(value: usize, buf_len: usize) -> Result<Vec<u8>, Error> {
        let mut storage = [0u8; 4];
        let mut out = &mut storage[..buf_len];
        let initial_len = out.len();
        let res = encode_berlen(&mut out, value)?;
        assert_eq!(out.len(), initial_len - res.len(), "cursor advancement");
        Ok(res.to_vec())
    }

    fn dec_len(input: &[u8]) -> Result<(usize, Vec<u8>), Error> {
        let mut inp = input;
        let len = decode_berlen(&mut inp)?;
        Ok((len, inp.to_vec()))
    }

    #[test]
    fn test_encode_bertag() {
        // Valid: 0-4 byte tags
        assert_eq!(enc_tag(b"", 4), Ok(vec![]));
        assert_eq!(enc_tag(b"\x5A", 4), Ok(vec![0x5A]));
        assert_eq!(enc_tag(b"\x9F\x02", 4), Ok(vec![0x9F, 0x02]));
        assert_eq!(enc_tag(b"\x9F\x81\x02", 4), Ok(vec![0x9F, 0x81, 0x02]));
        assert_eq!(enc_tag(b"\x9F\x81\x81\x02", 4), Ok(vec![0x9F, 0x81, 0x81, 0x02]));
        // Buffer overflow
        assert_eq!(enc_tag(b"\x5A", 0), Err(Error::BufferOverflow));
        assert_eq!(enc_tag(b"\x9F\x02", 1), Err(Error::BufferOverflow));
        assert_eq!(enc_tag(b"\x9F\x81\x02", 2), Err(Error::BufferOverflow));
        assert_eq!(enc_tag(b"\x9F\x81\x81\x02", 3), Err(Error::BufferOverflow));
    }

    #[test]
    fn test_decode_bertag() {
        // Single-byte (low 5 bits != 0x1F): boundaries
        assert_eq!(dec_tag(b"\x00"), Ok((vec![0x00], vec![])));
        assert_eq!(dec_tag(b"\x5A"), Ok((vec![0x5A], vec![])));
        assert_eq!(dec_tag(b"\xFE"), Ok((vec![0xFE], vec![])));
        // Two-byte: all 8 high-tag first bytes (& 0x1F == 0x1F)
        assert_eq!(dec_tag(b"\x1F\x01"), Ok((vec![0x1F, 0x01], vec![])));
        assert_eq!(dec_tag(b"\x3F\x01"), Ok((vec![0x3F, 0x01], vec![])));
        assert_eq!(dec_tag(b"\x5F\x01"), Ok((vec![0x5F, 0x01], vec![])));
        assert_eq!(dec_tag(b"\x7F\x01"), Ok((vec![0x7F, 0x01], vec![])));
        assert_eq!(dec_tag(b"\x9F\x01"), Ok((vec![0x9F, 0x01], vec![])));
        assert_eq!(dec_tag(b"\xBF\x01"), Ok((vec![0xBF, 0x01], vec![])));
        assert_eq!(dec_tag(b"\xDF\x01"), Ok((vec![0xDF, 0x01], vec![])));
        assert_eq!(dec_tag(b"\xFF\x01"), Ok((vec![0xFF, 0x01], vec![])));
        assert_eq!(dec_tag(b"\x9F\x7F"), Ok((vec![0x9F, 0x7F], vec![])));
        // Three-byte: continuation/terminator byte boundaries
        assert_eq!(dec_tag(b"\x9F\x81\x00"), Ok((vec![0x9F, 0x81, 0x00], vec![])));
        assert_eq!(dec_tag(b"\x9F\x81\x7F"), Ok((vec![0x9F, 0x81, 0x7F], vec![])));
        assert_eq!(dec_tag(b"\x9F\xFF\x00"), Ok((vec![0x9F, 0xFF, 0x00], vec![])));
        assert_eq!(dec_tag(b"\x9F\xFF\x7F"), Ok((vec![0x9F, 0xFF, 0x7F], vec![])));
        // Four-byte: boundaries
        assert_eq!(dec_tag(b"\x9F\x81\x80\x00"), Ok((vec![0x9F, 0x81, 0x80, 0x00], vec![])));
        assert_eq!(dec_tag(b"\x9F\xFF\xFF\x7F"), Ok((vec![0x9F, 0xFF, 0xFF, 0x7F], vec![])));
        // Trailing data preserved
        assert_eq!(dec_tag(b"\x5A\x03\xAB"), Ok((vec![0x5A], vec![0x03, 0xAB])));
        assert_eq!(dec_tag(b"\x9F\x02\x05"), Ok((vec![0x9F, 0x02], vec![0x05])));
        // Invalid: zero tag-number payload in first subsequent octet
        for &b in &[0x1F, 0x3F, 0x5F, 0x7F, 0x9F, 0xBF, 0xDF, 0xFF] {
            assert_eq!(dec_tag(&[b, 0x00]), Err(Error::Invalid));
            assert_eq!(dec_tag(&[b, 0x80]), Err(Error::Invalid));
        }
        // Invalid: >4 bytes
        assert_eq!(dec_tag(b"\x9F\x81\x80\x80\x00"), Err(Error::Invalid));
        assert_eq!(dec_tag(b"\x1F\xFF\xFF\xFF\x7F"), Err(Error::Invalid));
        // UnexpectedEof: empty
        assert_eq!(dec_tag(b""), Err(Error::UnexpectedEof));
        // UnexpectedEof: all 8 high-tag first bytes alone
        for &b in &[0x1F, 0x3F, 0x5F, 0x7F, 0x9F, 0xBF, 0xDF, 0xFF] {
            assert_eq!(dec_tag(&[b]), Err(Error::UnexpectedEof));
        }
        // UnexpectedEof: incomplete multi-byte
        assert_eq!(dec_tag(b"\x9F\x81"), Err(Error::UnexpectedEof));
        assert_eq!(dec_tag(b"\x9F\x81\x80"), Err(Error::UnexpectedEof));
        assert_eq!(dec_tag(b"\x9F\x81\x80\x80"), Err(Error::Invalid)); // 4 cont = invalid
    }

    #[test]
    fn test_encode_berlen() {
        // Short form: 0-0x7F
        assert_eq!(enc_len(0, 4), Ok(vec![0x00]));
        assert_eq!(enc_len(0x7F, 4), Ok(vec![0x7F]));
        // Long form 1 byte: 0x80-0xFF
        assert_eq!(enc_len(0x80, 4), Ok(vec![0x81, 0x80]));
        assert_eq!(enc_len(0xFF, 4), Ok(vec![0x81, 0xFF]));
        // Long form 2 bytes: 0x100-0xFFFF
        assert_eq!(enc_len(0x100, 4), Ok(vec![0x82, 0x01, 0x00]));
        assert_eq!(enc_len(0xFFFF, 4), Ok(vec![0x82, 0xFF, 0xFF]));
        // Invalid: too large
        assert_eq!(enc_len(0x1_0000, 4), Err(Error::Invalid));
        // Buffer overflow
        assert_eq!(enc_len(0x7F, 0), Err(Error::BufferOverflow));
        assert_eq!(enc_len(0x80, 1), Err(Error::BufferOverflow));
        assert_eq!(enc_len(0x100, 2), Err(Error::BufferOverflow));
    }

    #[test]
    fn test_decode_berlen() {
        // Short form boundaries
        assert_eq!(dec_len(b"\x00"), Ok((0, vec![])));
        assert_eq!(dec_len(b"\x7F"), Ok((0x7F, vec![])));
        // Long form 1 byte
        assert_eq!(dec_len(b"\x81\x80"), Ok((0x80, vec![])));
        assert_eq!(dec_len(b"\x81\xFF"), Ok((0xFF, vec![])));
        // Long form 2 bytes
        assert_eq!(dec_len(b"\x82\x01\x00"), Ok((0x100, vec![])));
        assert_eq!(dec_len(b"\x82\xFF\xFF"), Ok((0xFFFF, vec![])));
        // Trailing data preserved
        assert_eq!(dec_len(b"\x7F\xAA"), Ok((0x7F, vec![0xAA])));
        assert_eq!(dec_len(b"\x81\xFF\xBB"), Ok((0xFF, vec![0xBB])));
        assert_eq!(dec_len(b"\x82\x01\x00\xCC"), Ok((0x100, vec![0xCC])));
        // Non-minimal BER encodings (valid in BER)
        assert_eq!(dec_len(b"\x81\x00"), Ok((0, vec![])));
        assert_eq!(dec_len(b"\x81\x7F"), Ok((0x7F, vec![])));
        assert_eq!(dec_len(b"\x82\x00\x00"), Ok((0, vec![])));
        assert_eq!(dec_len(b"\x82\x00\x7F"), Ok((0x7F, vec![])));
        assert_eq!(dec_len(b"\x82\x00\xFF"), Ok((0xFF, vec![])));
        // Invalid: indefinite form
        assert_eq!(dec_len(b"\x80"), Err(Error::Invalid));
        assert_eq!(dec_len(b"\x80\x00"), Err(Error::Invalid));
        // Invalid: unsupported long forms (3+ length octets)
        for &b in &[0x83, 0x84, 0xFE, 0xFF] {
            assert_eq!(dec_len(&[b, 0, 0, 0, 0]), Err(Error::Invalid));
        }
        // UnexpectedEof
        assert_eq!(dec_len(b""), Err(Error::UnexpectedEof));
        assert_eq!(dec_len(b"\x81"), Err(Error::UnexpectedEof));
        assert_eq!(dec_len(b"\x82"), Err(Error::UnexpectedEof));
        assert_eq!(dec_len(b"\x82\x01"), Err(Error::UnexpectedEof));
    }

    #[test]
    fn test_sequential_decode() {
        let mut input: &[u8] = b"\x9F\x02\x02\xAB\xCD";
        assert_eq!(decode_bertag(&mut input), Ok(&b"\x9F\x02"[..]));
        assert_eq!(decode_berlen(&mut input), Ok(2));
        assert_eq!(input, b"\xAB\xCD");
    }

    #[test]
    fn test_parse_hex_tag_uppercase_only() {
        assert_eq!(parse_hex_tag("9F02"), Ok(([0x9F, 0x02, 0, 0], 2)));
        assert_eq!(parse_hex_tag(""), Err(Error::Internal));
        assert_eq!(parse_hex_tag("9f02"), Err(Error::Internal));
        assert_eq!(parse_hex_tag("9F0"), Err(Error::Internal));
        assert_eq!(parse_hex_tag("9G02"), Err(Error::Internal));
    }

    #[test]
    fn test_parse_unknown_tag_key_uppercase_only() {
        assert_eq!(parse_unknown_tag_key("t9F02_unknown"), Ok(([0x9F, 0x02, 0, 0], 2)));
        assert_eq!(parse_unknown_tag_key("t9f02_unknown"), Err(Error::Internal));
        assert_eq!(parse_unknown_tag_key("9F02_unknown"), Err(Error::Invalid));
        assert_eq!(parse_unknown_tag_key("t9F02"), Err(Error::Invalid));
    }
}

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[test]
        fn bertag_encode_roundtrip(tag in prop::collection::vec(any::<u8>(), 0..8)) {
            let mut buf = [0u8; 16];
            let mut out = &mut buf[..];
            let encoded = encode_bertag(&mut out, &tag).unwrap();
            prop_assert_eq!(encoded, tag.as_slice());
        }

        #[test]
        fn bertag_decode_valid_forms(class in 0u8..=3, constructed in any::<bool>(), tag_bytes in 0u8..3, term in 0u8..0x80) {
            let first = (class << 6) | (if constructed { 0x20 } else { 0 }) | 0x1F;
            let input: Vec<u8> = match tag_bytes {
                0 => vec![first, term.max(1)],
                1 => vec![first, 0x80 | (term >> 1).max(1), term],
                _ => vec![first, 0x80 | (term >> 2).max(1), 0x80 | (term >> 1), term],
            };
            let mut inp = &input[..];
            prop_assert_eq!(decode_bertag(&mut inp), Ok(&input[..]));
        }

        #[test]
        fn bertag_decode_too_long(class in 0u8..=3, c1 in 0x80u8..=0xFF, c2 in 0x80u8..=0xFF, c3 in 0x80u8..=0xFF, term in 0u8..0x80) {
            let first = (class << 6) | 0x1F;
            let mut inp: &[u8] = &[first, c1, c2, c3, term];
            prop_assert_eq!(decode_bertag(&mut inp), Err(Error::Invalid));
        }

        #[test]
        fn berlen_roundtrip(len in 0usize..=0xFFFF) {
            let mut buf = [0u8; 4];
            let mut out = &mut buf[..];
            let encoded = encode_berlen(&mut out, len).unwrap();
            let mut inp = &encoded[..];
            prop_assert_eq!(decode_berlen(&mut inp), Ok(len));
        }

        #[test]
        fn berlen_encode_rejects_large(len in 0x1_0000usize..=0xFF_FFFF) {
            let mut buf = [0u8; 8];
            let mut out = &mut buf[..];
            prop_assert_eq!(encode_berlen(&mut out, len), Err(Error::Invalid));
        }

        #[test]
        fn berlen_decode_non_minimal_long1(len in 0usize..=0xFF) {
            let mut inp: &[u8] = &[0x81, len as u8];
            prop_assert_eq!(decode_berlen(&mut inp), Ok(len));
        }

        #[test]
        fn berlen_decode_non_minimal_long2(len in 0usize..=0xFFFF) {
            let mut inp: &[u8] = &[0x82, (len >> 8) as u8, len as u8];
            prop_assert_eq!(decode_berlen(&mut inp), Ok(len));
        }
    }
}
