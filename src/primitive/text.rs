#[cfg(all(not(debug_assertions), feature = "no-panic"))]
use no_panic::no_panic;

use crate::Error;
use crate::primitive::bytes::reserve_bytes;
use crate::utils::cold_path;

/// Encode all input bytes, padding to at least `pad_to` bytes. Right alignment
/// adds padding on the left. Advances output and returns the written area.
/// Insufficient output returns `BufferOverflow` before writing or advancing.
/// Input longer than `pad_to` is preserved; use `truncate_bytes` for explicit loss.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn encode_bytes<'a>(
    output: &mut &'a mut [u8],
    input: &[u8],
    pad_to: usize,
    align_right: bool,
    padding: u8,
) -> Result<&'a mut [u8], Error> {
    let buf = reserve_bytes(output, input.len().max(pad_to))?;
    let pad_len = pad_to.saturating_sub(input.len());
    if align_right {
        for byte in buf.iter_mut().take(pad_len) {
            *byte = padding;
        }
        for (out, byte) in buf.iter_mut().skip(pad_len).zip(input) {
            *out = *byte;
        }
    } else {
        // Keep the explicit bound: small fixed fields copy inline instead of calling memcpy.
        for (out, byte) in buf.iter_mut().zip(input.iter().take(input.len())) {
            *out = *byte;
        }
        for byte in buf.iter_mut().skip(input.len()) {
            *byte = padding;
        }
    }
    Ok(buf)
}

/// Remove padding from an already framed byte slice, preserving at least
/// `min_len` bytes. Right alignment strips the left edge; otherwise the right.
/// Debug builds assert `min_len <= input.len()`; release stays bounded if violated.
/// Padding bytes within the protected minimum or at the other edge are retained.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn decode_bytes(mut input: &[u8], min_len: usize, align_right: bool, padding: u8) -> &[u8] {
    debug_assert!(min_len <= input.len(), "minimum exceeds field length");
    if align_right {
        while input.len() > min_len {
            match input {
                [byte, rest @ ..] if *byte == padding => input = rest,
                _ => break,
            }
        }
    } else {
        while input.len() > min_len {
            match input {
                [rest @ .., byte] if *byte == padding => input = rest,
                _ => break,
            }
        }
    }
    input
}

/// Retain at most `max_len` bytes from the left, or from the right when
/// `keep_right` is true. Returns a borrowed slice without copying.
/// Byte truncation may split UTF-8 characters; this is not Unicode truncation.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn truncate_bytes(input: &[u8], max_len: usize, keep_right: bool) -> &[u8] {
    let len = input.len().min(max_len);
    if keep_right { &input[input.len() - len..] } else { &input[..len] }
}

/// Apply the same byte limit to text, rejecting a cut inside a UTF-8 character.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub(crate) fn truncate_str_bytes(input: &str, max_len: usize, keep_right: bool) -> Result<&str, Error> {
    let len = truncate_bytes(input.as_bytes(), max_len, keep_right).len();
    let retained = if keep_right {
        input.get(input.len() - len..)
    } else {
        input.get(..len)
    };
    retained.ok_or_else(|| {
        cold_path();
        Error::Invalid
    })
}

/// Encode prevalidated ASCII text with byte padding, without truncation.
/// Debug builds assert the ASCII precondition; release does not validate it.
/// The padding byte may be non-ASCII.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn encode_ascii<'a>(
    output: &mut &'a mut [u8],
    input: &str,
    pad_to: usize,
    align_right: bool,
    padding: u8,
) -> Result<&'a mut [u8], Error> {
    debug_assert!(input.is_ascii(), "input must be ASCII");
    encode_bytes(output, input.as_bytes(), pad_to, align_right, padding)
}

/// Strip byte padding from a framed field and return its ASCII semantic text.
/// Debug builds assert the minimum-length and retained-ASCII preconditions.
/// Removed padding may use any byte value. Release performs only the UTF-8
/// conversion required for &str, returning `Invalid` for malformed UTF-8.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn decode_ascii(input: &[u8], min_len: usize, align_right: bool, padding: u8) -> Result<&str, Error> {
    let field = decode_bytes(input, min_len, align_right, padding);
    debug_assert!(field.is_ascii(), "retained text must be ASCII");
    str::from_utf8(field).map_err(|_| {
        cold_path();
        Error::Invalid
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitive::bytes::take_bytes;

    #[test]
    fn text_truncation_uses_byte_boundaries() {
        for input in ["", "ABC", "é", "Aé€🦀Z"] {
            for max_len in (0..=input.len() + 1).chain([usize::MAX]) {
                for keep_right in [false, true] {
                    let bytes = truncate_bytes(input.as_bytes(), max_len, keep_right);
                    let expected = str::from_utf8(bytes).map_err(|_| Error::Invalid);
                    let actual = truncate_str_bytes(input, max_len, keep_right);
                    assert_eq!(actual, expected);
                    if let Ok(actual) = actual {
                        assert_eq!(actual.as_ptr(), bytes.as_ptr());
                    }
                }
            }
        }
    }

    fn enc(input: &[u8], pad_to: usize, align_right: bool, padding: u8) -> Vec<u8> {
        let mut output = [0u8; 64];
        let mut out = output.as_mut_slice();
        let result = encode_bytes(&mut out, input, pad_to, align_right, padding).unwrap();
        assert_eq!(out.len(), 64 - result.len(), "cursor advancement");
        result.to_vec()
    }

    fn dec(input: &[u8], min_len: usize, align_right: bool, padding: u8) -> Vec<u8> {
        decode_bytes(input, min_len, align_right, padding).to_vec()
    }

    fn roundtrip(input: &[u8], pad_to: usize, align_right: bool, padding: u8) -> Vec<u8> {
        let encoded = enc(input, pad_to, align_right, padding);
        dec(&encoded, 0, align_right, padding)
    }

    #[test]
    fn test_encode() {
        // Empty input
        assert_eq!(enc(b"", 0, false, b' '), b"");
        assert_eq!(enc(b"", 5, false, b' '), b"     ");
        // Padding: left-aligned (pad right), right-aligned (pad left)
        assert_eq!(enc(b"Hi", 5, false, b' '), b"Hi   ");
        assert_eq!(enc(b"Hi", 5, true, b' '), b"   Hi");
        // No padding needed
        assert_eq!(enc(b"Hello", 5, false, b' '), b"Hello");
        // Truncation: left-aligned (keep left), right-aligned (keep right)
        assert_eq!(enc(truncate_bytes(b"HelloWorld", 5, false), 5, false, b' '), b"Hello");
        assert_eq!(enc(truncate_bytes(b"HelloWorld", 5, true), 5, true, b' '), b"World");
        // Different padding bytes
        assert_eq!(enc(b"X", 5, true, b'0'), b"0000X");
        assert_eq!(enc(b"X", 5, true, 0x00), b"\x00\x00\x00\x00X");
        assert_eq!(enc(b"X", 5, true, 0xFF), b"\xFF\xFF\xFF\xFFX");
        assert_eq!(enc(b"X", 5, true, 0x40), b"\x40\x40\x40\x40X"); // EBCDIC
        // High bytes in input
        assert_eq!(enc(b"\x80\x90\xA0", 5, true, b' '), b"  \x80\x90\xA0");
        assert_eq!(
            enc(truncate_bytes(b"\x80\x81\x82\x83\x84\x85", 3, true), 3, true, b' '),
            b"\x83\x84\x85"
        );
    }

    #[test]
    fn test_decode() {
        // Empty
        assert_eq!(dec(b"", 0, false, b' '), b"");
        // Strip leading (right-aligned) / trailing (left-aligned)
        assert_eq!(dec(b"   Hi", 0, true, b' '), b"Hi");
        assert_eq!(dec(b"Hi   ", 0, false, b' '), b"Hi");
        // All padding
        assert_eq!(dec(b"     ", 0, true, b' '), b"");
        assert_eq!(dec(b"     ", 0, false, b' '), b"");
        // No padding to strip
        assert_eq!(dec(b"Hello", 0, false, b' '), b"Hello");
        // minlen prevents over-stripping
        assert_eq!(dec(b"     ", 3, true, b' '), b"   ");
        assert_eq!(dec(b"  Hi", 3, true, b' '), b" Hi");
        // minlen == len (no stripping)
        assert_eq!(dec(b"   Hi", 5, true, b' '), b"   Hi");
        // Padding in middle - not stripped
        assert_eq!(dec(b"A B C", 0, false, b' '), b"A B C");
        // Padding at wrong end - not stripped
        assert_eq!(dec(b"Hi   ", 0, true, b' '), b"Hi   ");
        assert_eq!(dec(b"   Hi", 0, false, b' '), b"   Hi");
        // Different padding bytes
        assert_eq!(dec(b"\x00\x00AB", 0, true, 0x00), b"AB");
        assert_eq!(dec(b"\x40\x40Hi", 0, true, 0x40), b"Hi"); // EBCDIC
        // High bytes
        assert_eq!(dec(b"  \xFF\xFE", 0, true, b' '), b"\xFF\xFE");
        assert_eq!(dec(b"\xFF\xFF\xAB", 0, true, 0xFF), b"\xAB");
    }

    #[test]
    fn test_explicit_framing() {
        let mut input = b"Hi   tail".as_slice();
        let field = take_bytes(&mut input, 5).unwrap();
        assert_eq!(decode_bytes(field, 0, false, b' '), b"Hi");
        assert_eq!(input, b"tail");
        assert_eq!(take_bytes(&mut input, 5), Err(Error::UnexpectedEof));
        assert_eq!(input, b"tail");
    }

    #[test]
    fn test_roundtrip() {
        assert_eq!(roundtrip(b"Hello", 8, true, b' '), b"Hello");
        assert_eq!(roundtrip(b"Hello", 8, false, b' '), b"Hello");
        assert_eq!(roundtrip(b"Test", 4, true, b'0'), b"Test");
        assert_eq!(roundtrip(b"", 5, true, b' '), b"");
        assert_eq!(roundtrip(b"\x80\x90\xA0", 5, true, 0x40), b"\x80\x90\xA0");
    }

    #[test]
    fn test_ascii_wrappers() {
        // encode_ascii
        let mut out = [0u8; 64];
        let mut p = &mut out[..];
        assert_eq!(encode_ascii(&mut p, "Hello", 8, true, b' ').unwrap(), b"   Hello");
        // decode_ascii
        let inp: &[u8] = b"   Hello";
        assert_eq!(decode_ascii(inp, 0, true, b' ').unwrap(), "Hello");
    }

    #[test]
    #[cfg(not(debug_assertions))]
    fn test_decode_ascii_invalid_utf8() {
        let inp: &[u8] = b"\xFF\xFE";
        assert_eq!(decode_ascii(inp, 0, true, b' '), Err(Error::Invalid));
    }
    #[test]
    fn test_padding_and_truncation_boundaries() {
        for right in [false, true] {
            let mut storage = [0xA5; 8];
            let mut out = storage.as_mut_slice();
            assert_eq!(encode_bytes(&mut out, b"ABCDEF", 2, right, b' ').unwrap(), b"ABCDEF");
            assert_eq!(out, [0xA5; 2]);
            for (input, pad_to) in [(b"ABC".as_slice(), 0), (b"A", 3), (b"", usize::MAX)] {
                let mut out = &mut storage[..2];
                let before = out.to_vec();
                assert_eq!(encode_bytes(&mut out, input, pad_to, right, 0), Err(Error::BufferOverflow));
                assert_eq!(out, before);
            }
            assumption(|| {
                decode_bytes(b"ABC", 4, right, 0);
            });
            assumption(|| {
                decode_bytes(b"", usize::MAX, right, 0);
            });
            for (max_len, expected_len) in [(0, 0), (2, 2), (3, 3), (4, 3), (usize::MAX, 3)] {
                let input = b"ABC";
                let got = truncate_bytes(input, max_len, right);
                assert_eq!(got.len(), expected_len);
                assert_eq!(got.as_ptr(), input[if right { input.len() - expected_len } else { 0 }..].as_ptr());
            }
            assert!(core::str::from_utf8(truncate_bytes("é".as_bytes(), 1, right)).is_err());
        }
    }

    fn assumption(f: impl FnOnce()) {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        assert_eq!(result.is_err(), cfg!(debug_assertions));
    }

    #[test]
    fn test_ascii_assumptions_and_byte_padding() {
        for right in [false, true] {
            let mut storage = [0xA5; 8];
            assumption(|| {
                let _ = encode_ascii(&mut storage.as_mut_slice(), "é", 0, right, 0);
            });
            for input in ["é".as_bytes(), b"\xFF", b"A\xFF", "€".as_bytes()] {
                assumption(|| {
                    let _ = decode_ascii(input, 0, right, b' ');
                });
            }
            let encoded = encode_ascii(&mut storage.as_mut_slice(), "A\0\x7F", 5, right, 0xFF).unwrap();
            assert_eq!(decode_ascii(encoded, 3, right, 0xFF), Ok("A\0\x7F"));
            assumption(|| {
                let _ = decode_ascii(encoded, 5, right, 0xFF);
            });
        }
    }
}

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[test]
        fn encode_output_length(
            input in proptest::collection::vec(any::<u8>(), 0..50),
            pad_to in 0usize..20,
            align_right: bool,
            padding: u8,
        ) {
            let mut output = [0u8; 64];
            let mut outptr = &mut output[..];
            let result = encode_bytes(&mut outptr, &input, pad_to, align_right, padding).unwrap();
            prop_assert_eq!(result.len(), input.len().max(pad_to));
        }

        #[test]
        fn roundtrip_no_truncation(
            input in proptest::collection::vec(0x21u8..0x7F, 0..20),
            extra_padding in 0usize..10,
            align_right: bool,
        ) {
            let minlen = input.len() + extra_padding;
            let padding = b' ';
            let mut output = [0u8; 64];
            let mut outptr = &mut output[..];
            let encoded = encode_bytes(&mut outptr, &input, minlen, align_right, padding).unwrap();
            let decoded = decode_bytes(encoded, 0, align_right, padding);
            prop_assert_eq!(decoded, input.as_slice());
        }

        #[test]
        fn minlen_prevents_over_stripping(
            data in proptest::collection::vec(0x41u8..0x5B, 1..10),
            padding_count in 1usize..10,
            minlen_offset in 0usize..5,
            align_right: bool,
        ) {
            let padding = b' ';
            let total_len = data.len() + padding_count;
            let minlen = (data.len() + minlen_offset).min(total_len);
            let input = if align_right {
                let mut v = vec![padding; padding_count];
                v.extend(&data);
                v
            } else {
                let mut v = data.clone();
                v.extend(vec![padding; padding_count]);
                v
            };
            let decoded = decode_bytes(&input, minlen, align_right, padding);
            prop_assert!(decoded.len() >= minlen);
            if align_right {
                prop_assert!(decoded.ends_with(&data));
            } else {
                prop_assert!(decoded.starts_with(&data));
            }
        }
        #[test]
        fn protected_padding_roundtrips_any_bytes(
            input in prop::collection::vec(any::<u8>(), 0..64),
            extra in 0usize..32,
            right in any::<bool>(),
            padding in any::<u8>(),
        ) {
            let mut storage = [0xA5; 96];
            let mut output = storage.as_mut_slice();
            let encoded = encode_bytes(&mut output, &input, input.len() + extra, right, padding).unwrap();
            prop_assert_eq!(encoded.len(), input.len() + extra);
            prop_assert!(output.iter().all(|&b| b == 0xA5));
            let decoded = decode_bytes(encoded, input.len(), right, padding);
            prop_assert_eq!(decoded, input.as_slice());
            prop_assert_eq!(decoded.as_ptr(), encoded[if right { extra } else { 0 }..].as_ptr());
        }

        #[test]
        fn truncation_matches_selected_edge(input in prop::collection::vec(any::<u8>(), 0..64), limit in any::<usize>(), right in any::<bool>()) {
            let mut expected: Vec<_> = if right {
                input.iter().rev().take(limit).copied().collect()
            } else {
                input.iter().take(limit).copied().collect()
            };
            if right { expected.reverse(); }
            prop_assert_eq!(truncate_bytes(&input, limit, right), expected);
        }

    }
}
