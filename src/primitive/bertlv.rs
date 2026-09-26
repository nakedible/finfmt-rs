//! BER-TLV tags and definite lengths as used by EMV (EMV 4.4 Book 3, Annex B),
//! which applies the BER rules of ITU-T X.690 with two relaxations that
//! decoding accepts:
//!
//! - Long-form lengths need not be minimal (`81 05`, `82 00 05`). X.690
//!   8.1.3.5 leaves this to the sender; only DER requires the minimum.
//!   Encoding always writes the minimal form.
//! - Two-byte tags may carry tag numbers below 31 (`9F02`). X.690 8.1.2.2
//!   requires the one-byte form for those, but EMV defines such tags. The tag
//!   number must still be nonzero (EMV Table 40, X.690 8.1.2.4.2), so `1F 00`
//!   and `1F 80` are rejected.
//!
//! Supported ranges are limits of this library, not aliases: tags up to
//! [`MAX_BER_TAG_BYTES`] (EMV uses one or two bytes) and values up to
//! [`MAX_BER_VALUE_LEN`] bytes (EMV uses one to three length bytes).

#[cfg(all(not(debug_assertions), feature = "no-panic"))]
use no_panic::no_panic;

use crate::Error;
use crate::primitive::bytes::{copy_bytes, reserve_bytes, take_bytes};
use crate::primitive::nibble::{UpperHexDigits, pack_nibbles, pack_nibbles_checked, unpack_nibbles};
use crate::utils::cold_path;

/// Maximum encoded tag size accepted by this library's tag parsers.
pub const MAX_BER_TAG_BYTES: usize = 4;
/// Maximum length of a tag written as hex.
pub const MAX_BER_TAG_HEX: usize = 2 * MAX_BER_TAG_BYTES;
/// Maximum value length supported by this library's definite BER length codec.
pub const MAX_BER_VALUE_LEN: usize = u16::MAX as usize;

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
pub fn encode_ber_tag<'a>(output: &mut &'a mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
    copy_bytes(output, input)
}

/// Decodes a BER tag from `input`, advancing the slice to point after the tag.
///
/// Rules (high-tag-number form):
/// - One-octet tag if low 5 bits of the first octet are not `0x1F`.
/// - Otherwise, continuation octets are consumed until a byte with MSB 0 is found.
/// - Supports tags up to 4 octets; longer tags are rejected.
/// - Accepts financial tags such as `9F02`; no ASN.1 minimum numeric tag is imposed.
///
/// This frames tag octets only, including the `00` end-of-contents identifier.
/// [`decode_ber_tlv_entry`] rejects that identifier as an ordinary data entry.
///
/// Returns:
/// - A sub-slice of `input` that contains the tag bytes.
///
/// Errors:
/// - `Error::Invalid` if the tag would exceed 4 octets.
/// - `Error::UnexpectedEof` if `input` does not contain enough bytes.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn decode_ber_tag<'a>(input: &mut &'a [u8]) -> Result<&'a [u8], Error> {
    match *input {
        [a, ..] if *a & 0x1F != 0x1F => take_bytes(input, 1),
        [_, 0x01..=0x7F, ..] => take_bytes(input, 2),
        [_, 0x81..=0xFF, c, ..] if *c < 0x80 => take_bytes(input, 3),
        [_, 0x81..=0xFF, _, d, ..] if *d < 0x80 => take_bytes(input, 4),
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
pub fn encode_ber_length<'a>(output: &mut &'a mut [u8], input: usize) -> Result<&'a mut [u8], Error> {
    match input {
        0..=0x7F => copy_bytes(output, &[input as u8]),
        0x80..=0xFF => copy_bytes(output, &[0x81, input as u8]),
        0x0100..=MAX_BER_VALUE_LEN => copy_bytes(output, &[0x82, (input >> 8) as u8, input as u8]),
        _ => {
            cold_path();
            Err(Error::Invalid)
        }
    }
}

/// Decodes a definite-form BER length from `input`, advancing the slice.
///
/// Returns:
/// - The decoded length as `usize`. Supported non-minimal long forms are accepted.
///
/// Errors:
/// - `Error::Invalid` for indefinite form (`0x80`), reserved/unsupported forms
///   (`0x83..=0xFF`), or other invalid encodings.
/// - `Error::UnexpectedEof` if `input` does not contain enough bytes.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn decode_ber_length(input: &mut &[u8]) -> Result<usize, Error> {
    match *input {
        [a @ 0x00..=0x7F, ..] => {
            take_bytes(input, 1)?;
            Ok(*a as usize)
        }
        _ => decode_ber_length_long(input),
    }
}

/// Number of octets in the shortest supported definite BER length encoding.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn ber_length_width(len: usize) -> Result<usize, Error> {
    match len {
        0..=0x7F => Ok(1),
        0x80..=0xFF => Ok(2),
        0x0100..=MAX_BER_VALUE_LEN => Ok(3),
        _ => {
            cold_path();
            Err(Error::Invalid)
        }
    }
}

/// Parses uppercase hex representing exactly one supported data tag.
///
/// Returns `Invalid` for malformed text, incomplete or concatenated tags, and
/// the `00` end-of-contents identifier. Unused array bytes are zero.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn parse_ber_tag_hex(tag: &str) -> Result<([u8; MAX_BER_TAG_BYTES], usize), Error> {
    let bytes = tag.as_bytes();
    if bytes.is_empty() || bytes.len() > 2 * MAX_BER_TAG_BYTES || !bytes.len().is_multiple_of(2) {
        cold_path();
        return Err(Error::Invalid);
    }
    let mut out = [0u8; MAX_BER_TAG_BYTES];
    // The length check above means packing can only fail on a non-hex digit.
    let packed = pack_nibbles_checked::<UpperHexDigits>(&mut &mut out[..], bytes).map_err(|_| {
        cold_path();
        Error::Invalid
    })?;
    let mut input = &*packed;
    let tag = decode_ber_tag(&mut input).map_err(|_| {
        cold_path();
        Error::Invalid
    })?;
    if !input.is_empty() || tag == [0] {
        cold_path();
        return Err(Error::Invalid);
    }
    Ok((out, bytes.len() / 2))
}

/// Write a framed tag as uppercase hex, the form of textual tag literals, so an
/// entry's tag is formatted once and compared with each literal as bytes.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn format_ber_tag_hex<'a>(output: &'a mut [u8; MAX_BER_TAG_HEX], tag: &[u8]) -> &'a [u8] {
    debug_assert!(tag.len() <= MAX_BER_TAG_BYTES, "a framed BER tag has at most four bytes");
    let mut out = &mut output[..];
    unpack_nibbles::<UpperHexDigits>(&mut out, tag).map_or(&[], |hex| &*hex)
}

/// Pack a tag literal written in uppercase hex, the inverse of
/// [`format_ber_tag_hex`]. The literal is trusted: validate it with
/// [`parse_ber_tag_hex`] where it is not known to be valid.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn pack_ber_tag_hex<'a>(output: &'a mut [u8; MAX_BER_TAG_BYTES], tag_hex: &str) -> &'a [u8] {
    debug_assert!(
        parse_ber_tag_hex(tag_hex).is_ok(),
        "a BER tag literal must be a valid tag in uppercase hex"
    );
    let mut out = &mut output[..];
    pack_nibbles::<UpperHexDigits>(&mut out, tag_hex.as_bytes(), false, 0).map_or(&[], |tag| &*tag)
}

/// Write the tag and definite length of an entry whose value length is known,
/// advancing the cursor so the value can be encoded directly after them.
/// Tag bytes are copied without validation. Insufficient capacity returns
/// `BufferOverflow`; unsupported value lengths return `Invalid`.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn encode_ber_tlv_head<'a>(output: &mut &'a mut [u8], tag: &[u8], value_len: usize) -> Result<&'a mut [u8], Error> {
    let head_len = tag.len().checked_add(ber_length_width(value_len)?).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    let head = reserve_bytes(output, head_len)?;
    let mut cursor = &mut *head;
    encode_ber_tag(&mut cursor, tag)?;
    encode_ber_length(&mut cursor, value_len)?;
    Ok(head)
}

/// Borrowed tag and value bytes; the original length octets are not retained.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BerTlvEntry<'a> {
    pub tag: &'a [u8],
    pub value: &'a [u8],
}

/// Frames one definite-length data entry, or returns `None` at clean empty input.
/// Rejects end-of-contents as data and does not skip padding.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn decode_ber_tlv_entry<'a>(input: &mut &'a [u8]) -> Result<Option<BerTlvEntry<'a>>, Error> {
    if input.is_empty() {
        return Ok(None);
    }
    let tag = decode_ber_tag(input)?;
    if tag == [0] {
        cold_path();
        return Err(Error::Invalid);
    }
    let len = decode_ber_length(input)?;
    let value = take_bytes(input, len)?;
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
fn decode_ber_length_long(input: &mut &[u8]) -> Result<usize, Error> {
    match *input {
        [0x81, b, ..] => {
            take_bytes(input, 2)?;
            Ok(*b as usize)
        }
        [0x82, b, c, ..] => {
            take_bytes(input, 3)?;
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
        let res = encode_ber_tag(&mut out, input)?;
        assert_eq!(out.len(), initial_len - res.len(), "cursor advancement");
        Ok(res.to_vec())
    }

    fn dec_tag(input: &[u8]) -> Result<(Vec<u8>, Vec<u8>), Error> {
        let mut inp = input;
        let tag = decode_ber_tag(&mut inp)?;
        Ok((tag.to_vec(), inp.to_vec()))
    }

    fn enc_len(value: usize, buf_len: usize) -> Result<Vec<u8>, Error> {
        let mut storage = [0u8; 4];
        let mut out = &mut storage[..buf_len];
        let initial_len = out.len();
        let res = encode_ber_length(&mut out, value)?;
        assert_eq!(out.len(), initial_len - res.len(), "cursor advancement");
        Ok(res.to_vec())
    }

    fn dec_len(input: &[u8]) -> Result<(usize, Vec<u8>), Error> {
        let mut inp = input;
        let len = decode_ber_length(&mut inp)?;
        Ok((len, inp.to_vec()))
    }

    #[test]
    fn tag_literals_pack_and_format_as_inverses() {
        for text in ["5A", "9F02", "9F8101", "DF818001"] {
            let (expected, len) = parse_ber_tag_hex(text).unwrap();
            let mut packed = [0; MAX_BER_TAG_BYTES];
            let tag = pack_ber_tag_hex(&mut packed, text);
            assert_eq!(tag, &expected[..len]);
            let mut hex = [0; MAX_BER_TAG_HEX];
            assert_eq!(format_ber_tag_hex(&mut hex, tag), text.as_bytes());
        }
    }

    #[test]
    fn test_encode_ber_tlv_head() {
        for (tag, len, head) in [
            (&b"\x5A"[..], 0, &b"\x5A\x00"[..]),
            (b"\x9F\x02", 127, b"\x9F\x02\x7F"),
            (b"\x9F\x02", 128, b"\x9F\x02\x81\x80"),
            (b"\xDF\x81\x81\x01", 256, b"\xDF\x81\x81\x01\x82\x01\x00"),
        ] {
            for capacity in [head.len() - 1, head.len(), head.len() + 3] {
                let mut storage = [0xAA; 16];
                let mut output = &mut storage[..capacity];
                let result = encode_ber_tlv_head(&mut output, tag, len).map(|written| written.to_vec());
                if capacity < head.len() {
                    assert_eq!(result, Err(Error::BufferOverflow));
                } else {
                    assert_eq!(result, Ok(head.to_vec()));
                    assert_eq!(output.len(), capacity - head.len());
                }
            }
        }
        assert_eq!(
            encode_ber_tlv_head(&mut &mut [0; 8][..], b"\x5A", MAX_BER_VALUE_LEN + 1),
            Err(Error::Invalid)
        );
    }

    #[test]
    fn test_encode_ber_tag() {
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
    fn test_decode_ber_tag() {
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
    fn test_encode_ber_length() {
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
    fn test_decode_ber_length() {
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
        assert_eq!(decode_ber_tag(&mut input), Ok(&b"\x9F\x02"[..]));
        assert_eq!(decode_ber_length(&mut input), Ok(2));
        assert_eq!(input, b"\xAB\xCD");
    }

    #[test]
    fn test_parse_ber_tag_hex_uppercase_only() {
        assert_eq!(parse_ber_tag_hex("9F02"), Ok(([0x9F, 0x02, 0, 0], 2)));
        for (text, expected, len) in [
            ("5A", [0x5A, 0, 0, 0], 1),
            ("9F817F", [0x9F, 0x81, 0x7F, 0], 3),
            ("FF818000", [0xFF, 0x81, 0x80, 0], 4),
        ] {
            assert_eq!(parse_ber_tag_hex(text), Ok((expected, len)));
            let mut hex = [0; MAX_BER_TAG_HEX];
            assert_eq!(format_ber_tag_hex(&mut hex, &expected[..len]), text.as_bytes());
        }
        for text in [
            "00",
            "9F",
            "9F81",
            "9F8180",
            "5A5B",
            "9F025A",
            "9F00",
            "9F8001",
            "9F818080",
            "9F81808000",
            "é",
        ] {
            assert_eq!(parse_ber_tag_hex(text), Err(Error::Invalid), "{text}");
        }
        assert_eq!(parse_ber_tag_hex(""), Err(Error::Invalid));
        assert_eq!(parse_ber_tag_hex("9f02"), Err(Error::Invalid));
        assert_eq!(parse_ber_tag_hex("9F0"), Err(Error::Invalid));
        assert_eq!(parse_ber_tag_hex("9G02"), Err(Error::Invalid));
    }

    #[test]
    fn test_raw_entry_boundaries() {
        assert_eq!(decode_ber_tlv_entry(&mut &b""[..]), Ok(None));
        for wire in [&b"\x5A\x00"[..], &b"\x5A\x81\x00"[..], &b"\x5A\x82\x00\x00"[..]] {
            let mut input = wire;
            assert_eq!(decode_ber_tlv_entry(&mut input), Ok(Some(BerTlvEntry { tag: &[0x5A], value: &[] })));
            assert!(input.is_empty());
        }
        for wire in [&b"\x00"[..], &b"\x00\x00"[..], &b"\x5A\x80\x00\x00"[..], &b"\x5A\x83"[..]] {
            assert_eq!(decode_ber_tlv_entry(&mut &wire[..]), Err(Error::Invalid));
        }
        let wire = b"\x9F\x81\x80\x00\x82\x00\x02\xAB\xCD";
        for len in 1..wire.len() {
            assert_eq!(decode_ber_tlv_entry(&mut &wire[..len]), Err(Error::UnexpectedEof));
        }
        let mut input = &b"\x9F\x02\x02\xAB\xCD\x5A\x00"[..];
        assert_eq!(
            decode_ber_tlv_entry(&mut input),
            Ok(Some(BerTlvEntry {
                tag: &[0x9F, 0x02],
                value: &[0xAB, 0xCD]
            }))
        );
        assert_eq!(decode_ber_tlv_entry(&mut input), Ok(Some(BerTlvEntry { tag: &[0x5A], value: &[] })));
        assert_eq!(decode_ber_tlv_entry(&mut input), Ok(None));
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
            let encoded = encode_ber_tag(&mut out, &tag).unwrap();
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
            prop_assert_eq!(decode_ber_tag(&mut inp), Ok(&input[..]));
            let text: String = input.iter().map(|byte| format!("{byte:02X}")).collect();
            let (tag, len) = parse_ber_tag_hex(&text).unwrap();
            prop_assert_eq!(&tag[..len], input.as_slice());
            let mut hex = [0; MAX_BER_TAG_HEX];
            prop_assert_eq!(format_ber_tag_hex(&mut hex, &input), text.as_bytes());
        }

        #[test]
        fn bertag_decode_too_long(class in 0u8..=3, c1 in 0x80u8..=0xFF, c2 in 0x80u8..=0xFF, c3 in 0x80u8..=0xFF, term in 0u8..0x80) {
            let first = (class << 6) | 0x1F;
            let mut inp: &[u8] = &[first, c1, c2, c3, term];
            prop_assert_eq!(decode_ber_tag(&mut inp), Err(Error::Invalid));
        }

        #[test]
        fn berlen_roundtrip(len in 0usize..=0xFFFF) {
            let mut buf = [0u8; 4];
            let mut out = &mut buf[..];
            let encoded = encode_ber_length(&mut out, len).unwrap();
            let mut inp = &encoded[..];
            prop_assert_eq!(decode_ber_length(&mut inp), Ok(len));
            prop_assert_eq!(ber_length_width(len), Ok(encoded.len()));
        }

        #[test]
        fn berlen_encode_rejects_large(len in 0x1_0000usize..=0xFF_FFFF) {
            let mut buf = [0u8; 8];
            let mut out = &mut buf[..];
            prop_assert_eq!(encode_ber_length(&mut out, len), Err(Error::Invalid));
        }

        #[test]
        fn berlen_decode_non_minimal_long1(len in 0usize..=0xFF) {
            let mut inp: &[u8] = &[0x81, len as u8];
            prop_assert_eq!(decode_ber_length(&mut inp), Ok(len));
        }

        #[test]
        fn berlen_decode_non_minimal_long2(len in 0usize..=0xFFFF) {
            let mut inp: &[u8] = &[0x82, (len >> 8) as u8, len as u8];
            prop_assert_eq!(decode_ber_length(&mut inp), Ok(len));
        }
    }
}
