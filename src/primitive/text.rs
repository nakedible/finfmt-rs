#[cfg(all(not(debug_assertions), feature = "no-panic"))]
use no_panic::no_panic;

use crate::Error;
use crate::utils::cold_path;

/// Encode bytes into `output` with padding and optional truncation.
///
/// Behavior:
/// - Pads with `padding` to at least `minlen` bytes.
/// - Truncates to at most `maxlen` bytes.
/// - If `align_right` is `true`, padding is added on the left and truncation
///   removes from the left (keeping the rightmost bytes). Otherwise padding and
///   truncation are applied on the right.
///
/// The written portion of `output` is returned and `output` is advanced by the
/// number of bytes written.
///
/// Works with any single-byte encoding (ASCII, EBCDIC, ISO-8859-1, etc.).
/// Use the appropriate padding byte for your encoding (e.g., 0x20 for ASCII
/// space, 0x40 for EBCDIC space).
///
/// Errors:
/// - `Error::BufferOverflow` if `output` has insufficient capacity.
///
/// Panics:
/// - In debug builds if `minlen > maxlen`.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn encode_bytes<'a>(
    output: &mut &'a mut [u8],
    input: &[u8],
    minlen: usize,
    maxlen: usize,
    align_right: bool,
    padding: u8,
) -> Result<&'a mut [u8], Error> {
    debug_assert!(minlen <= maxlen, "minlen must be less than or equal to maxlen");
    let copylen = input.len().min(maxlen);
    let padlen = minlen.saturating_sub(copylen);
    let buf = output.split_off_mut(..(padlen + copylen)).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    if align_right {
        for b in buf.iter_mut().take(padlen) {
            *b = padding;
        }
        for (o, i) in buf.iter_mut().skip(padlen).zip(input.iter().rev().take(copylen).rev()) {
            *o = *i;
        }
    } else {
        for (o, i) in buf.iter_mut().zip(input.iter().take(copylen)) {
            *o = *i;
        }
        for b in buf.iter_mut().skip(copylen) {
            *b = padding;
        }
    }
    Ok(buf)
}

/// Decode a padded field from `input` into bytes.
///
/// Reads exactly `len` bytes from `input`, then removes leading or trailing
/// `padding` bytes so the result is at least `minlen` bytes long. If
/// `align_right` is `true`, padding is assumed on the left; otherwise on the
/// right. No padding is removed when `minlen == len`.
///
/// The returned slice is a view into the consumed portion of `input`.
///
/// Works with any single-byte encoding (ASCII, EBCDIC, ISO-8859-1, etc.).
/// Use the appropriate padding byte for your encoding.
///
/// Errors:
/// - `Error::UnexpectedEof` if fewer than `len` bytes are available.
///
/// Panics:
/// - In debug builds if `minlen > len`.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn decode_bytes<'a>(input: &mut &'a [u8], minlen: usize, len: usize, align_right: bool, padding: u8) -> Result<&'a [u8], Error> {
    debug_assert!(minlen <= len, "minlen must be less than or equal to len");
    let mut field = input.split_off(..len).ok_or_else(|| {
        cold_path();
        Error::UnexpectedEof
    })?;
    if minlen < len {
        if align_right {
            let padding_len = field
                .iter()
                .position(|&b| b != padding)
                .unwrap_or(field.len())
                .min(field.len().saturating_sub(minlen));
            let skip = padding_len.min(field.len());
            let (_, remaining) = field.split_at(skip);
            field = remaining;
        } else {
            let data_len = field
                .iter()
                .rposition(|&b| b != padding)
                .map(|pos| pos + 1)
                .unwrap_or(0)
                .max(minlen);
            let keep = data_len.min(field.len());
            let (kept, _) = field.split_at(keep);
            field = kept;
        }
    }
    Ok(field)
}

/// Encode an ASCII string into `output` with padding and optional truncation.
///
/// This is a convenience wrapper around [`encode_bytes`] that takes `&str`.
///
/// Errors:
/// - `Error::BufferOverflow` if `output` has insufficient capacity.
///
/// Panics:
/// - In debug builds if `minlen > maxlen`.
/// - In debug builds if `input` contains non-ASCII bytes.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn encode_ascii<'a>(
    output: &mut &'a mut [u8],
    input: &str,
    minlen: usize,
    maxlen: usize,
    align_right: bool,
    padding: u8,
) -> Result<&'a mut [u8], Error> {
    debug_assert!(input.is_ascii(), "Input must be ASCII");
    encode_bytes(output, input.as_bytes(), minlen, maxlen, align_right, padding)
}

/// Decode a padded ASCII field from `input` into `&str`.
///
/// This is a convenience wrapper around [`decode_bytes`] that asserts the
/// result is ASCII in debug builds and converts to `&str`.
///
/// Errors:
/// - `Error::UnexpectedEof` if fewer than `len` bytes are available.
/// - `Error::Invalid` if the bytes are not valid UTF-8.
///
/// Panics:
/// - In debug builds if `minlen > len`.
/// - In debug builds if the field contains non-ASCII bytes.
#[inline(always)]
// `#[no_panic]` is disabled here because the Rust standard library's
// `str::from_utf8` path cannot currently be proven panic-free by `no-panic`.
pub fn decode_ascii<'a>(input: &mut &'a [u8], minlen: usize, len: usize, align_right: bool, padding: u8) -> Result<&'a str, Error> {
    let field = decode_bytes(input, minlen, len, align_right, padding)?;
    debug_assert!(field.is_ascii(), "Field must be ASCII");
    str::from_utf8(field).map_err(|_| {
        cold_path();
        Error::Invalid
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enc(input: &[u8], minlen: usize, maxlen: usize, align_right: bool, padding: u8) -> Vec<u8> {
        let mut output = [0u8; 64];
        let initial_len = output.len();
        let mut outptr = &mut output[..];
        let result = encode_bytes(&mut outptr, input, minlen, maxlen, align_right, padding).unwrap();
        assert_eq!(outptr.len(), initial_len - result.len(), "cursor advancement");
        result.to_vec()
    }

    fn dec(input: &[u8], minlen: usize, len: usize, align_right: bool, padding: u8) -> Vec<u8> {
        let mut inptr = input;
        let initial_len = inptr.len();
        let result = decode_bytes(&mut inptr, minlen, len, align_right, padding).unwrap();
        assert_eq!(inptr.len(), initial_len - len, "cursor advancement");
        result.to_vec()
    }

    fn enc_err(input: &[u8], buf_len: usize, minlen: usize, maxlen: usize) -> Result<(), Error> {
        let mut output = [0u8; 64];
        let mut outptr = &mut output[..buf_len];
        encode_bytes(&mut outptr, input, minlen, maxlen, false, b' ').map(|_| ())
    }

    fn dec_err(input: &[u8], len: usize) -> Result<(), Error> {
        let mut inptr = input;
        decode_bytes(&mut inptr, 0, len, true, b' ').map(|_| ())
    }

    fn roundtrip(input: &[u8], minlen: usize, maxlen: usize, align_right: bool, padding: u8) -> Vec<u8> {
        let mut output = [0u8; 64];
        let mut outptr = &mut output[..];
        let encoded = encode_bytes(&mut outptr, input, minlen, maxlen, align_right, padding).unwrap();
        let mut encoded_ref: &[u8] = encoded;
        decode_bytes(&mut encoded_ref, 0, encoded.len(), align_right, padding)
            .unwrap()
            .to_vec()
    }

    #[test]
    fn test_encode() {
        // Empty input
        assert_eq!(enc(b"", 0, 0, false, b' '), b"");
        assert_eq!(enc(b"", 5, 5, false, b' '), b"     ");
        // Padding: left-aligned (pad right), right-aligned (pad left)
        assert_eq!(enc(b"Hi", 5, 10, false, b' '), b"Hi   ");
        assert_eq!(enc(b"Hi", 5, 10, true, b' '), b"   Hi");
        // No padding needed
        assert_eq!(enc(b"Hello", 5, 10, false, b' '), b"Hello");
        // Truncation: left-aligned (keep left), right-aligned (keep right)
        assert_eq!(enc(b"HelloWorld", 5, 5, false, b' '), b"Hello");
        assert_eq!(enc(b"HelloWorld", 5, 5, true, b' '), b"World");
        // Different padding bytes
        assert_eq!(enc(b"X", 5, 5, true, b'0'), b"0000X");
        assert_eq!(enc(b"X", 5, 5, true, 0x00), b"\x00\x00\x00\x00X");
        assert_eq!(enc(b"X", 5, 5, true, 0xFF), b"\xFF\xFF\xFF\xFFX");
        assert_eq!(enc(b"X", 5, 5, true, 0x40), b"\x40\x40\x40\x40X"); // EBCDIC
        // High bytes in input
        assert_eq!(enc(b"\x80\x90\xA0", 5, 5, true, b' '), b"  \x80\x90\xA0");
        assert_eq!(enc(b"\x80\x81\x82\x83\x84\x85", 3, 3, true, b' '), b"\x83\x84\x85");
    }

    #[test]
    fn test_decode() {
        // Empty
        assert_eq!(dec(b"", 0, 0, false, b' '), b"");
        // Strip leading (right-aligned) / trailing (left-aligned)
        assert_eq!(dec(b"   Hi", 0, 5, true, b' '), b"Hi");
        assert_eq!(dec(b"Hi   ", 0, 5, false, b' '), b"Hi");
        // All padding
        assert_eq!(dec(b"     ", 0, 5, true, b' '), b"");
        assert_eq!(dec(b"     ", 0, 5, false, b' '), b"");
        // No padding to strip
        assert_eq!(dec(b"Hello", 0, 5, false, b' '), b"Hello");
        // minlen prevents over-stripping
        assert_eq!(dec(b"     ", 3, 5, true, b' '), b"   ");
        assert_eq!(dec(b"  Hi", 3, 4, true, b' '), b" Hi");
        // minlen == len (no stripping)
        assert_eq!(dec(b"   Hi", 5, 5, true, b' '), b"   Hi");
        // Padding in middle - not stripped
        assert_eq!(dec(b"A B C", 0, 5, false, b' '), b"A B C");
        // Padding at wrong end - not stripped
        assert_eq!(dec(b"Hi   ", 0, 5, true, b' '), b"Hi   ");
        assert_eq!(dec(b"   Hi", 0, 5, false, b' '), b"   Hi");
        // Different padding bytes
        assert_eq!(dec(b"\x00\x00AB", 0, 4, true, 0x00), b"AB");
        assert_eq!(dec(b"\x40\x40Hi", 0, 4, true, 0x40), b"Hi"); // EBCDIC
        // High bytes
        assert_eq!(dec(b"  \xFF\xFE", 0, 4, true, b' '), b"\xFF\xFE");
        assert_eq!(dec(b"\xFF\xFF\xAB", 0, 3, true, 0xFF), b"\xAB");
    }

    #[test]
    fn test_errors() {
        // BufferOverflow: buffer too small, zero buffer
        assert_eq!(enc_err(b"Test", 3, 5, 5), Err(Error::BufferOverflow));
        assert_eq!(enc_err(b"X", 0, 1, 1), Err(Error::BufferOverflow));
        // UnexpectedEof
        assert_eq!(dec_err(b"ABC", 5), Err(Error::UnexpectedEof));
        assert_eq!(dec_err(b"", 1), Err(Error::UnexpectedEof));
    }

    #[test]
    fn test_roundtrip() {
        assert_eq!(roundtrip(b"Hello", 8, 8, true, b' '), b"Hello");
        assert_eq!(roundtrip(b"Hello", 8, 8, false, b' '), b"Hello");
        assert_eq!(roundtrip(b"Test", 4, 10, true, b'0'), b"Test");
        assert_eq!(roundtrip(b"", 5, 5, true, b' '), b"");
        assert_eq!(roundtrip(b"\x80\x90\xA0", 5, 5, true, 0x40), b"\x80\x90\xA0");
    }

    #[test]
    fn test_ascii_wrappers() {
        // encode_ascii
        let mut out = [0u8; 64];
        let mut p = &mut out[..];
        assert_eq!(encode_ascii(&mut p, "Hello", 8, 8, true, b' ').unwrap(), b"   Hello");
        // decode_ascii
        let mut inp: &[u8] = b"   Hello";
        assert_eq!(decode_ascii(&mut inp, 0, 8, true, b' ').unwrap(), "Hello");
    }

    #[test]
    #[cfg(not(debug_assertions))]
    fn test_decode_ascii_invalid_utf8() {
        let mut inp: &[u8] = b"\xFF\xFE";
        assert_eq!(decode_ascii(&mut inp, 0, 2, true, b' '), Err(Error::Invalid));
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
            minlen in 0usize..20,
            maxlen in 0usize..20,
            align_right: bool,
            padding: u8,
        ) {
            let (minlen, maxlen) = if minlen <= maxlen { (minlen, maxlen) } else { (maxlen, minlen) };
            let mut output = [0u8; 64];
            let mut outptr = &mut output[..];
            let result = encode_bytes(&mut outptr, &input, minlen, maxlen, align_right, padding).unwrap();
            prop_assert_eq!(result.len(), input.len().min(maxlen).max(minlen));
        }

        #[test]
        fn roundtrip_no_truncation(
            input in proptest::collection::vec(0x21u8..0x7F, 0..20),
            extra_padding in 0usize..10,
            align_right: bool,
        ) {
            let minlen = input.len() + extra_padding;
            let maxlen = minlen + 10;
            let padding = b' ';
            let mut output = [0u8; 64];
            let mut outptr = &mut output[..];
            let encoded = encode_bytes(&mut outptr, &input, minlen, maxlen, align_right, padding).unwrap();
            let mut encoded_ref: &[u8] = encoded;
            let decoded = decode_bytes(&mut encoded_ref, 0, encoded.len(), align_right, padding).unwrap();
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
            let mut inptr: &[u8] = &input;
            let decoded = decode_bytes(&mut inptr, minlen, total_len, align_right, padding).unwrap();
            prop_assert!(decoded.len() >= minlen);
            if align_right {
                prop_assert!(decoded.ends_with(&data));
            } else {
                prop_assert!(decoded.starts_with(&data));
            }
        }
    }
}
