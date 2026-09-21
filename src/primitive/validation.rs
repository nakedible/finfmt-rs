//! Content validators. Functions taking `minlen` and `maxlen` require
//! `minlen <= maxlen`; debug builds assert this caller invariant.
//! With valid bounds, content errors take precedence over length errors.

use std::ops::RangeBounds;

#[cfg(all(not(debug_assertions), feature = "no-panic"))]
use no_panic::no_panic;

use crate::Error;
use crate::primitive::ebcdic::{EBCDIC_037_TO_ASCII, encode_ebcdic_1142_char};
use crate::utils::cold_path;

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
fn validate_bytes(input: impl AsRef<[u8]>, minlen: usize, maxlen: usize, pred: impl Fn(&u8) -> bool) -> Result<usize, Error> {
    debug_assert!(minlen <= maxlen, "minlen must be <= maxlen");
    let input = input.as_ref();
    if !input.iter().all(pred) {
        cold_path();
        return Err(Error::Invalid);
    }
    if input.len() < minlen || input.len() > maxlen {
        cold_path();
        return Err(Error::InvalidValueLength);
    }
    Ok(input.len())
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
fn validate_chars(input: &str, minlen: usize, maxlen: usize, pred: impl Fn(char) -> bool) -> Result<usize, Error> {
    debug_assert!(minlen <= maxlen, "minlen must be <= maxlen");
    let count = input.chars().try_fold(0, |acc, c| {
        pred(c).then_some(acc + 1).ok_or_else(|| {
            cold_path();
            Error::Invalid
        })
    })?;
    if count < minlen || count > maxlen {
        cold_path();
        return Err(Error::InvalidValueLength);
    }
    Ok(count)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
fn validate_even_bytes(input: impl AsRef<[u8]>, minlen: usize, maxlen: usize, pred: impl Fn(&u8) -> bool) -> Result<usize, Error> {
    let len = validate_bytes(input, minlen, maxlen, pred)?;
    if !len.is_multiple_of(2) {
        cold_path();
        return Err(Error::InvalidValueLength);
    }
    Ok(len)
}

/// Validate ASCII decimal digits, returning their byte count.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_numeric(input: impl AsRef<[u8]>, minlen: usize, maxlen: usize) -> Result<usize, Error> {
    validate_bytes(input, minlen, maxlen, u8::is_ascii_digit)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_alpha(input: impl AsRef<[u8]>, minlen: usize, maxlen: usize) -> Result<usize, Error> {
    validate_bytes(input, minlen, maxlen, u8::is_ascii_alphabetic)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_alphanum(input: impl AsRef<[u8]>, minlen: usize, maxlen: usize) -> Result<usize, Error> {
    validate_bytes(input, minlen, maxlen, u8::is_ascii_alphanumeric)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_ascii(input: impl AsRef<[u8]>, minlen: usize, maxlen: usize) -> Result<usize, Error> {
    validate_bytes(input, minlen, maxlen, u8::is_ascii)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_ascii_printable(input: impl AsRef<[u8]>, minlen: usize, maxlen: usize) -> Result<usize, Error> {
    validate_bytes(input, minlen, maxlen, |b| matches!(b, b' '..=b'~'))
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_upper_alpha(input: impl AsRef<[u8]>, minlen: usize, maxlen: usize) -> Result<usize, Error> {
    validate_bytes(input, minlen, maxlen, |b| b.is_ascii_uppercase())
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_upper_alphanum(input: impl AsRef<[u8]>, minlen: usize, maxlen: usize) -> Result<usize, Error> {
    validate_bytes(input, minlen, maxlen, |b| b.is_ascii_digit() || b.is_ascii_uppercase())
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_upper_ascii_printable(input: impl AsRef<[u8]>, minlen: usize, maxlen: usize) -> Result<usize, Error> {
    validate_bytes(input, minlen, maxlen, |b| matches!(b, b' '..=b'~') && !b.is_ascii_lowercase())
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_hex(input: impl AsRef<[u8]>, minlen: usize, maxlen: usize) -> Result<usize, Error> {
    validate_bytes(input, minlen, maxlen, u8::is_ascii_hexdigit)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_hex_upper(input: impl AsRef<[u8]>, minlen: usize, maxlen: usize) -> Result<usize, Error> {
    validate_bytes(input, minlen, maxlen, |b| matches!(b, b'0'..=b'9' | b'A'..=b'F'))
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_hex_lower(input: impl AsRef<[u8]>, minlen: usize, maxlen: usize) -> Result<usize, Error> {
    validate_bytes(input, minlen, maxlen, |b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_hex_even(input: impl AsRef<[u8]>, minlen: usize, maxlen: usize) -> Result<usize, Error> {
    validate_even_bytes(input, minlen, maxlen, u8::is_ascii_hexdigit)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_hex_upper_even(input: impl AsRef<[u8]>, minlen: usize, maxlen: usize) -> Result<usize, Error> {
    validate_even_bytes(input, minlen, maxlen, |b| matches!(b, b'0'..=b'9' | b'A'..=b'F'))
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_hex_lower_even(input: impl AsRef<[u8]>, minlen: usize, maxlen: usize) -> Result<usize, Error> {
    validate_even_bytes(input, minlen, maxlen, |b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// Validate expanded BCD-Z bytes (`0` through `?`), returning the byte count.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_bcdz(input: impl AsRef<[u8]>, minlen: usize, maxlen: usize) -> Result<usize, Error> {
    validate_bytes(input, minlen, maxlen, |b| matches!(b, b'0'..=b'?'))
}

/// Validate ASCII digits and `=` only, returning the byte count.
/// This checks the character set, not Track 2 separator placement or structure.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_track2_chars(input: impl AsRef<[u8]>, minlen: usize, maxlen: usize) -> Result<usize, Error> {
    validate_bytes(input, minlen, maxlen, |b| matches!(b, b'0'..=b'9' | b'='))
}

/// Validate packed decimal nibbles, returning the byte count. Input is already
/// encoded bytes; a string argument is inspected as UTF-8 bytes without conversion.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_bcd_bytes(input: impl AsRef<[u8]>, minlen: usize, maxlen: usize) -> Result<usize, Error> {
    validate_bytes(input, minlen, maxlen, |b| (b >> 4) <= 9 && (b & 0x0F) <= 9)
}

/// Validate the semantic byte length without restricting byte values.
/// Out-of-range lengths return `InvalidValueLength`.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_byte_length(input: impl AsRef<[u8]>, minlen: usize, maxlen: usize) -> Result<usize, Error> {
    validate_bytes(input, minlen, maxlen, |_| true)
}

/// Validate Unicode characters representable in Latin-1, returning the character
/// count. The input is a Rust string; the `Iso88591` field instead accepts raw bytes.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_iso8859_1_str(input: &str, minlen: usize, maxlen: usize) -> Result<usize, Error> {
    validate_chars(input, minlen, maxlen, |c| (c as u32) <= 0xFF)
}

/// Validate UTF-8 text representable in IBM1142, returning its Unicode character
/// count. Invalid UTF-8 or unrepresentable characters return `Invalid`.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_ebcdic_1142_text(input: impl AsRef<[u8]>, minlen: usize, maxlen: usize) -> Result<usize, Error> {
    let input = input.as_ref();
    debug_assert!(minlen <= maxlen, "minlen must be <= maxlen");
    if input.is_ascii() {
        let len = input.len();
        if len < minlen || len > maxlen {
            cold_path();
            return Err(Error::InvalidValueLength);
        }
        return Ok(len);
    }
    let text = core::str::from_utf8(input).map_err(|_| {
        cold_path();
        Error::Invalid
    })?;
    validate_chars(text, minlen, maxlen, |ch| encode_ebcdic_1142_char(ch).is_some())
}

/// Validate CP037 wire bytes representing ASCII characters, including controls,
/// and return their byte count. Use before lossy CP037-to-ASCII conversion when
/// unsupported characters must be rejected instead of replaced with SUB.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_ebcdic_037_ascii(input: impl AsRef<[u8]>, minlen: usize, maxlen: usize) -> Result<usize, Error> {
    validate_bytes(input, minlen, maxlen, |&byte| {
        // Only canonical CP037 SUB may map to ASCII SUB without substitution.
        EBCDIC_037_TO_ASCII[byte as usize] != 0x1A || byte == 0x3F
    })
}

/// Validate the EBCDIC byte range 0x40..=0xFE, returning the byte count. For
/// CP037 and IBM1142 this is the non-control repertoire, including space,
/// non-breaking space and soft hyphen; it does not imply ASCII representability.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_ebcdic_printable(input: impl AsRef<[u8]>, minlen: usize, maxlen: usize) -> Result<usize, Error> {
    validate_bytes(input, minlen, maxlen, |b| (0x40..=0xFE).contains(b))
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_range<T: Ord>(input: T, range: impl RangeBounds<T>) -> Result<(), Error> {
    if !range.contains(&input) {
        cold_path();
        return Err(Error::Invalid);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: validate with unbounded length
    fn vb<T: AsRef<[u8]>>(input: T, pred: impl Fn(&u8) -> bool) -> Result<usize, Error> {
        validate_bytes(input, 0, usize::MAX, pred)
    }

    #[test]
    fn test_validate_bytes_length() {
        // Length checks
        assert_eq!(validate_bytes("abc", 1, 3, |_| true), Ok(3));
        assert_eq!(validate_bytes("a", 1, 3, |_| true), Ok(1));
        assert_eq!(validate_bytes("", 1, 3, |_| true), Err(Error::InvalidValueLength));
        assert_eq!(validate_bytes("abcd", 1, 3, |_| true), Err(Error::InvalidValueLength));
        assert_eq!(validate_bytes("abc", 4, 5, |_| true), Err(Error::InvalidValueLength));
        // Predicate checked before length (Invalid returned even if length wrong)
        assert_eq!(validate_bytes("abc", 1, 3, |_| false), Err(Error::Invalid));
        assert_eq!(validate_bytes("abc", 1, 2, |_| false), Err(Error::Invalid));
        assert_eq!(validate_bytes("abc", 4, 5, |_| false), Err(Error::Invalid));
        // Custom predicate
        assert_eq!(vb("02468", |b| b % 2 == 0), Ok(5));
        assert_eq!(vb("02568", |b| b % 2 == 0), Err(Error::Invalid));
    }

    #[test]
    fn test_validate_chars_length() {
        // Counts chars, not bytes
        assert_eq!(validate_chars("abc", 1, 3, |_| true), Ok(3));
        assert_eq!(validate_chars("abcd", 1, 3, |_| true), Err(Error::InvalidValueLength));
        assert_eq!(validate_chars("héllo", 1, 5, |_| true), Ok(5)); // 6 bytes, 5 chars
        assert_eq!(validate_chars("héllo", 1, 4, |_| true), Err(Error::InvalidValueLength));
        assert_eq!(validate_chars("こんにちは", 1, 5, |_| true), Ok(5)); // 15 bytes, 5 chars
        assert_eq!(validate_chars("こんにちは", 1, 4, |_| true), Err(Error::InvalidValueLength));
    }

    #[test]
    fn test_validate_numeric() {
        assert_eq!(validate_numeric("0123456789", 0, 99), Ok(10));
        assert_eq!(validate_numeric(b"0123456789", 0, 99), Ok(10)); // &[u8]
        assert_eq!(validate_numeric("", 0, 99), Ok(0));
        assert_eq!(validate_numeric("", 1, 99), Err(Error::InvalidValueLength));
        assert_eq!(validate_numeric("123", 4, 5), Err(Error::InvalidValueLength));
        assert_eq!(validate_numeric("123456", 1, 5), Err(Error::InvalidValueLength));
        // Invalid chars
        for s in ["12a", " 12", "1.2", "-1", "1 2"] {
            assert_eq!(validate_numeric(s, 0, 99), Err(Error::Invalid));
        }
    }

    #[test]
    fn test_validate_alpha() {
        assert_eq!(validate_alpha("abcXYZ", 0, 99), Ok(6));
        assert_eq!(validate_alpha("", 0, 99), Ok(0));
        assert_eq!(validate_alpha("abc", 4, 5), Err(Error::InvalidValueLength));
        for s in ["abc1", "a b", "a.b"] {
            assert_eq!(validate_alpha(s, 0, 99), Err(Error::Invalid));
        }
    }

    #[test]
    fn test_validate_alphanum() {
        assert_eq!(validate_alphanum("abc123XYZ", 0, 99), Ok(9));
        assert_eq!(validate_alphanum("", 0, 99), Ok(0));
        for s in ["abc 123", "abc.123", "abc-123"] {
            assert_eq!(validate_alphanum(s, 0, 99), Err(Error::Invalid));
        }
    }

    #[test]
    fn test_validate_ascii() {
        assert_eq!(validate_ascii("hello\x00\x7F", 0, 99), Ok(7)); // boundaries
        assert_eq!(validate_ascii("hello", 10, 20), Err(Error::InvalidValueLength));
        assert_eq!(validate_ascii("héllo", 0, 99), Err(Error::Invalid));
        assert_eq!(validate_ascii("こんにちは", 0, 99), Err(Error::Invalid));
        assert_eq!(validate_ascii(b"\x00\x7F", 0, 99), Ok(2));
        assert_eq!(validate_ascii("", 0, 99), Ok(0));
        assert_eq!(validate_ascii(b"hello", 10, 20), Err(Error::InvalidValueLength));
        assert_eq!(validate_ascii(b"\x80", 0, 99), Err(Error::Invalid));
        assert_eq!(validate_ascii(b"\xFF", 0, 99), Err(Error::Invalid));
    }

    #[test]
    fn test_validate_ascii_printable() {
        // Boundaries: 0x20 (space) to 0x7E (~)
        assert_eq!(validate_ascii_printable(" ", 0, 99), Ok(1));
        assert_eq!(validate_ascii_printable("~", 0, 99), Ok(1));
        assert_eq!(validate_ascii_printable("Hello World!~!@#$%^&*()", 0, 99), Ok(23));
        assert_eq!(validate_ascii_printable("", 0, 99), Ok(0));
        // Invalid: control chars, DEL
        for s in ["\x1f", "\t", "\n", "\x7f"] {
            assert_eq!(validate_ascii_printable(s, 0, 99), Err(Error::Invalid));
        }
    }

    #[test]
    fn test_validate_upper_classes() {
        assert_eq!(validate_upper_alpha("ABC", 0, 99), Ok(3));
        assert_eq!(validate_upper_alpha("", 0, 99), Ok(0));
        assert_eq!(validate_upper_alpha("AB1", 0, 99), Err(Error::Invalid));
        assert_eq!(validate_upper_alpha("AbC", 0, 99), Err(Error::Invalid));
        assert_eq!(validate_upper_alphanum("ABC123", 0, 99), Ok(6));
        assert_eq!(validate_upper_alphanum("ABC-123", 0, 99), Err(Error::Invalid));
        assert_eq!(validate_upper_alphanum("AbC123", 0, 99), Err(Error::Invalid));
        assert_eq!(validate_upper_ascii_printable("ABC 123-=/", 0, 99), Ok(10));
        assert_eq!(validate_upper_ascii_printable("", 0, 99), Ok(0));
        assert_eq!(validate_upper_ascii_printable("AbC 123", 0, 99), Err(Error::Invalid));
        assert_eq!(validate_upper_ascii_printable("\n", 0, 99), Err(Error::Invalid));
    }

    #[test]
    fn test_validate_hex() {
        assert_eq!(validate_hex("0123456789abcdefABCDEF", 0, 99), Ok(22));
        assert_eq!(validate_hex("", 0, 99), Ok(0));
        assert_eq!(validate_hex("0g", 0, 99), Err(Error::Invalid));
        assert_eq!(validate_hex(" 0", 0, 99), Err(Error::Invalid));
    }

    #[test]
    fn test_validate_hex_upper() {
        assert_eq!(validate_hex_upper("0123456789ABCDEF", 0, 99), Ok(16));
        assert_eq!(validate_hex_upper("abcdef", 0, 99), Err(Error::Invalid));
    }

    #[test]
    fn test_validate_hex_lower() {
        assert_eq!(validate_hex_lower("0123456789abcdef", 0, 99), Ok(16));
        assert_eq!(validate_hex_lower("ABCDEF", 0, 99), Err(Error::Invalid));
    }

    #[test]
    fn test_validate_hex_even() {
        assert_eq!(validate_hex_even("01", 0, 99), Ok(2));
        assert_eq!(validate_hex_even("0123abCD", 0, 99), Ok(8));
        assert_eq!(validate_hex_even("", 0, 99), Ok(0));
        assert_eq!(validate_hex_even("012", 0, 99), Err(Error::InvalidValueLength)); // odd
        assert_eq!(validate_hex_even("0g", 0, 99), Err(Error::Invalid));
    }

    #[test]
    fn test_validate_hex_upper_even() {
        assert_eq!(validate_hex_upper_even("0123ABCD", 0, 99), Ok(8));
        assert_eq!(validate_hex_upper_even("012", 0, 99), Err(Error::InvalidValueLength));
        assert_eq!(validate_hex_upper_even("0123abcd", 0, 99), Err(Error::Invalid));
    }

    #[test]
    fn test_validate_hex_lower_even() {
        assert_eq!(validate_hex_lower_even("0123abcd", 0, 99), Ok(8));
        assert_eq!(validate_hex_lower_even("012", 0, 99), Err(Error::InvalidValueLength));
        assert_eq!(validate_hex_lower_even("0123ABCD", 0, 99), Err(Error::Invalid));
    }

    #[test]
    fn test_validate_bcdz() {
        assert_eq!(validate_bcdz("0123456789:;<=>?", 0, 99), Ok(16)); // full range
        assert_eq!(validate_bcdz("", 0, 99), Ok(0));
        assert_eq!(validate_bcdz("@", 0, 99), Err(Error::Invalid)); // above
        assert_eq!(validate_bcdz("/", 0, 99), Err(Error::Invalid)); // below
    }

    #[test]
    fn test_validate_track2() {
        assert_eq!(validate_track2_chars("1234567890=", 0, 99), Ok(11));
        assert_eq!(validate_track2_chars("", 0, 99), Ok(0));
        assert_eq!(validate_track2_chars("1234D", 0, 99), Err(Error::Invalid));
        assert_eq!(validate_track2_chars("12?", 0, 99), Err(Error::Invalid));
        assert_eq!(validate_track2_chars("123", 4, 5), Err(Error::InvalidValueLength));
    }

    #[test]
    fn test_validate_bcd_bytes() {
        // Valid: each nibble 0-9
        assert_eq!(validate_bcd_bytes(b"\x00\x12\x34\x56\x78\x99", 0, 99), Ok(6));
        assert_eq!(validate_bcd_bytes(b"\x00", 0, 99), Ok(1)); // min
        assert_eq!(validate_bcd_bytes(b"\x99", 0, 99), Ok(1)); // max
        assert_eq!(validate_bcd_bytes(b"", 0, 99), Ok(0));
        // Invalid nibbles
        assert_eq!(validate_bcd_bytes(b"\x0A", 0, 99), Err(Error::Invalid)); // low nibble A
        assert_eq!(validate_bcd_bytes(b"\xA0", 0, 99), Err(Error::Invalid)); // high nibble A
        assert_eq!(validate_bcd_bytes(b"\x9A", 0, 99), Err(Error::Invalid)); // low nibble A
        assert_eq!(validate_bcd_bytes(b"\xFF", 0, 99), Err(Error::Invalid)); // both F
    }

    #[test]
    fn test_validate_binary() {
        assert_eq!(validate_byte_length(b"\x00\xFF\x80\x7F", 0, 99), Ok(4)); // all bytes valid
        assert_eq!(validate_byte_length(b"", 0, 99), Ok(0));
        assert_eq!(validate_byte_length(b"hello", 10, 20), Err(Error::InvalidValueLength));
    }

    #[test]
    fn test_validate_iso8859_1_str() {
        assert_eq!(validate_iso8859_1_str("hello", 0, 99), Ok(5));
        assert_eq!(validate_iso8859_1_str("héllo", 0, 99), Ok(5)); // é is Latin-1
        assert_eq!(validate_iso8859_1_str("ÿ", 0, 99), Ok(1)); // U+00FF, max Latin-1
        assert_eq!(validate_iso8859_1_str("hello", 10, 20), Err(Error::InvalidValueLength));
        assert_eq!(validate_iso8859_1_str("Ā", 0, 99), Err(Error::Invalid)); // U+0100
        assert_eq!(validate_iso8859_1_str("こんにちは", 0, 99), Err(Error::Invalid));
    }

    #[test]
    fn test_validate_ebcdic_1142_text() {
        assert_eq!(validate_ebcdic_1142_text("ABC".as_bytes(), 0, 99), Ok(3));
        assert_eq!(validate_ebcdic_1142_text("ABCÆØÅæøå€".as_bytes(), 0, 99), Ok(10));
        assert_eq!(
            validate_ebcdic_1142_text("ABCÆØÅæøå€".as_bytes(), 0, 9),
            Err(Error::InvalidValueLength)
        );
        assert_eq!(validate_ebcdic_1142_text("emoji: 😀".as_bytes(), 0, 99), Err(Error::Invalid));
        assert_eq!(validate_ebcdic_1142_text([0xFF], 0, 99), Err(Error::Invalid));
    }

    #[test]
    fn test_validate_binary_alias_for_iso8859_1_bytes() {
        assert_eq!(validate_byte_length(b"\x00\x7F\x80\xFF", 0, 99), Ok(4));
        assert_eq!(validate_byte_length(b"", 0, 99), Ok(0));
        assert_eq!(validate_byte_length(b"hello", 10, 20), Err(Error::InvalidValueLength));
    }

    #[test]
    fn test_validate_ebcdic_037_ascii() {
        use crate::primitive::ebcdic::ASCII_TO_EBCDIC_037;

        for byte in 0..=u8::MAX {
            let expected = if ASCII_TO_EBCDIC_037[..128].contains(&byte) {
                Ok(1)
            } else {
                Err(Error::Invalid)
            };
            assert_eq!(validate_ebcdic_037_ascii([byte], 1, 1), expected, "wire byte {byte:02X}");
        }
        assert_eq!(validate_ebcdic_037_ascii([0xC1, 0xF1, 0x40], 3, 3), Ok(3));
        assert_eq!(validate_ebcdic_037_ascii("\0\x3F", 2, 2), Ok(2));
        assert_eq!(validate_ebcdic_037_ascii(b"\0\x3F", 2, 2), Ok(2));
        assert_eq!(validate_ebcdic_037_ascii(b"", 0, 0), Ok(0));
        assert_eq!(validate_ebcdic_037_ascii(b"", 1, 1), Err(Error::InvalidValueLength));
        assert_eq!(validate_ebcdic_037_ascii([0xC1], 2, 2), Err(Error::InvalidValueLength));
        assert_eq!(validate_ebcdic_037_ascii([0xC1], 0, 0), Err(Error::InvalidValueLength));
        for input in [[0x4A, 0xC1], [0xC1, 0x4A]] {
            for (min, max) in [(0, 0), (2, 2), (3, 3)] {
                assert_eq!(validate_ebcdic_037_ascii(input, min, max), Err(Error::Invalid));
            }
        }
    }

    #[test]
    fn test_validate_ebcdic_printable() {
        // Range: 0x40-0xFE
        assert_eq!(validate_ebcdic_printable(b"\x40", 0, 99), Ok(1)); // min
        assert_eq!(validate_ebcdic_printable(b"\xFE", 0, 99), Ok(1)); // max
        assert_eq!(validate_ebcdic_printable(b"\x40\xC1\xFE", 0, 99), Ok(3));
        assert_eq!(validate_ebcdic_printable(b"", 0, 99), Ok(0));
        assert_eq!(validate_ebcdic_printable(b"\x3F", 0, 99), Err(Error::Invalid)); // below
        assert_eq!(validate_ebcdic_printable(b"\xFF", 0, 99), Err(Error::Invalid)); // above
        assert_eq!(validate_ebcdic_printable(b"\x00", 0, 99), Err(Error::Invalid));
    }

    #[test]
    fn test_validate_range() {
        // Exclusive range
        assert_eq!(validate_range(5, 1..10), Ok(()));
        assert_eq!(validate_range(1, 1..10), Ok(())); // start inclusive
        assert_eq!(validate_range(9, 1..10), Ok(())); // end exclusive
        assert_eq!(validate_range(10, 1..10), Err(Error::Invalid));
        assert_eq!(validate_range(0, 1..10), Err(Error::Invalid));
        // Inclusive range
        assert_eq!(validate_range(10, 1..=10), Ok(()));
        assert_eq!(validate_range(11, 1..=10), Err(Error::Invalid));
        // Open ranges
        assert_eq!(validate_range(5, ..10), Ok(()));
        assert_eq!(validate_range(5, 5..), Ok(()));
        assert_eq!(validate_range(4, 5..), Err(Error::Invalid));
        // Char ranges
        assert_eq!(validate_range('c', 'a'..'{'), Ok(()));
        assert_eq!(validate_range('{', 'a'..'{'), Err(Error::Invalid));
    }
    #[test]
    fn test_even_hex_content_precedes_length() {
        assert_eq!(validate_hex_upper_even("a", 0, 10), Err(Error::Invalid));
        assert_eq!(validate_hex_lower_even("A", 0, 10), Err(Error::Invalid));
        type Validator = fn(&[u8], usize, usize) -> Result<usize, Error>;
        let validators: [Validator; 3] = [
            |s, min, max| validate_hex_even(s, min, max),
            |s, min, max| validate_hex_upper_even(s, min, max),
            |s, min, max| validate_hex_lower_even(s, min, max),
        ];
        for validate in validators {
            for bad in [b"G".as_slice(), b"0G1", b"GG", b"\xFF"] {
                for (min, max) in [(0, 0), (0, 10), (10, 10)] {
                    assert_eq!(validate(bad, min, max), Err(Error::Invalid));
                }
            }
            for valid_odd in [b"0".as_slice(), b"001"] {
                assert_eq!(validate(valid_odd, 0, 10), Err(Error::InvalidValueLength));
            }
            assert_eq!(validate(b"01", 2, 2), Ok(2));
            assert_eq!(validate(b"01", 3, 4), Err(Error::InvalidValueLength));
            assert_eq!(validate(b"01", 0, 1), Err(Error::InvalidValueLength));
            assert_eq!(validate(b"", 0, 0), Ok(0));
        }
    }

    #[test]
    fn test_input_representations_and_length_units() {
        assert_eq!(validate_byte_length("é", 2, 2), Ok(2));
        assert_eq!(validate_byte_length("é".as_bytes(), 2, 2), Ok(2));
        assert_eq!(validate_iso8859_1_str("é", 1, 1), Ok(1));
        assert_eq!(validate_iso8859_1_str("Ā", 10, 10), Err(Error::Invalid));
        assert_eq!(validate_bcd_bytes("\x12", 1, 1), Ok(1));
        assert_eq!(validate_bcd_bytes(b"\x12", 1, 1), Ok(1));
        assert_eq!(validate_ebcdic_printable("A", 1, 1), Ok(1));
        assert_eq!(validate_ebcdic_printable(b"\x41\xCA", 2, 2), Ok(2));
        assert_eq!(validate_ebcdic_1142_text("A€", 2, 2), Ok(2));
        assert_eq!(validate_ebcdic_1142_text("A€".as_bytes(), 2, 2), Ok(2));
        for invalid in ["¤".as_bytes(), "😀".as_bytes(), b"\x80"] {
            for (min, max) in [(0, 0), (0, 10), (10, 10)] {
                assert_eq!(validate_ebcdic_1142_text(invalid, min, max), Err(Error::Invalid));
            }
        }
        assert_eq!(validate_ebcdic_1142_text("A€", 0, 1), Err(Error::InvalidValueLength));
        assert_eq!(validate_ebcdic_1142_text("A€", 3, 3), Err(Error::InvalidValueLength));
        assert_eq!(validate_ebcdic_1142_text("", 0, 0), Ok(0));
    }
}

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        // Character class validation: arbitrary inputs, validate expected result
        #[test]
        fn numeric_validation(s in ".{0,50}") {
            let valid = s.bytes().all(|b| b.is_ascii_digit());
            let result = validate_numeric(&s, 0, usize::MAX);
            prop_assert_eq!(result.is_ok(), valid);
            if valid { prop_assert_eq!(result.unwrap(), s.len()); }
        }

        #[test]
        fn alpha_validation(s in ".{0,50}") {
            let valid = s.bytes().all(|b| b.is_ascii_alphabetic());
            let result = validate_alpha(&s, 0, usize::MAX);
            prop_assert_eq!(result.is_ok(), valid);
            if valid { prop_assert_eq!(result.unwrap(), s.len()); }
        }

        #[test]
        fn alphanum_validation(s in ".{0,50}") {
            let valid = s.bytes().all(|b| b.is_ascii_alphanumeric());
            let result = validate_alphanum(&s, 0, usize::MAX);
            prop_assert_eq!(result.is_ok(), valid);
        }

        #[test]
        fn ascii_validation(v in proptest::collection::vec(any::<u8>(), 0..100)) {
            let valid = v.iter().all(u8::is_ascii);
            let result = validate_ascii(&v, 0, usize::MAX);
            prop_assert_eq!(result.is_ok(), valid);
            if valid { prop_assert_eq!(result.unwrap(), v.len()); }
        }

        #[test]
        fn ascii_printable_validation(v in proptest::collection::vec(any::<u8>(), 0..100)) {
            let valid = v.iter().all(|&b| (0x20..=0x7E).contains(&b));
            let result = validate_ascii_printable(&v, 0, usize::MAX);
            prop_assert_eq!(result.is_ok(), valid);
        }

        // Hex case sensitivity: ensures lowercase rejected when uppercase expected and vice versa
        #[test]
        fn hex_upper_rejects_lowercase(s in "[0-9A-F]*[a-f]+[0-9A-F]*") {
            prop_assert!(validate_hex_upper(&s, 0, usize::MAX).is_err());
        }

        #[test]
        fn hex_lower_rejects_uppercase(s in "[0-9a-f]*[A-F]+[0-9a-f]*") {
            prop_assert!(validate_hex_lower(&s, 0, usize::MAX).is_err());
        }

        // Hex even: odd length always fails regardless of chars
        #[test]
        fn hex_even_rejects_odd(s in "[0-9a-fA-F]{1,51}") {
            let odd = if s.len() % 2 == 1 { s.clone() } else { s[..s.len() - 1].to_string() };
            prop_assume!(!odd.is_empty());
            prop_assert_eq!(validate_hex_even(&odd, 0, usize::MAX), Err(Error::InvalidValueLength));
        }

        // BCD bytes: valid nibbles (0-9) vs invalid nibbles (A-F)
        #[test]
        fn bcd_bytes_validation(v in proptest::collection::vec(any::<u8>(), 0..50)) {
            let valid = v.iter().all(|b| (b >> 4) <= 9 && (b & 0x0F) <= 9);
            let result = validate_bcd_bytes(&v, 0, usize::MAX);
            prop_assert_eq!(result.is_ok(), valid);
        }

        // BCDZ extended range: 0x30-0x3F ('0'-'?')
        #[test]
        fn bcdz_validation(v in proptest::collection::vec(any::<u8>(), 0..50)) {
            let valid = v.iter().all(|&b| (b'0'..=b'?').contains(&b));
            let result = validate_bcdz(&v, 0, usize::MAX);
            prop_assert_eq!(result.is_ok(), valid);
        }

        #[test]
        fn track2_validation(v in proptest::collection::vec(any::<u8>(), 0..50)) {
            let valid = v.iter().all(|&b| b.is_ascii_digit() || b == b'=');
            let result = validate_track2_chars(&v, 0, usize::MAX);
            prop_assert_eq!(result.is_ok(), valid);
        }

        // Binary accepts everything
        #[test]
        fn binary_accepts_all(v in proptest::collection::vec(any::<u8>(), 0..100)) {
            prop_assert_eq!(validate_byte_length(&v, 0, usize::MAX), Ok(v.len()));
        }

        // ISO-8859-1 str: char count vs byte count
        #[test]
        fn iso8859_1_char_counting(v in proptest::collection::vec(0u8..=255, 0..100)) {
            let s: String = v.iter().map(|&b| b as char).collect();
            prop_assert_eq!(validate_iso8859_1_str(&s, 0, usize::MAX), Ok(s.chars().count()));
        }

        #[test]
        fn ebcdic_037_ascii_validation(v in prop::collection::vec(any::<u8>(), 0..100), min in 0usize..=100, extra in 0usize..=100) {
            use crate::primitive::ebcdic::ASCII_TO_EBCDIC_037;
            let max = min + extra;
            let expected = if !v.iter().all(|b| ASCII_TO_EBCDIC_037[..128].contains(b)) {
                Err(Error::Invalid)
            } else if !(min..=max).contains(&v.len()) {
                Err(Error::InvalidValueLength)
            } else {
                Ok(v.len())
            };
            prop_assert_eq!(validate_ebcdic_037_ascii(&v, min, max), expected);
        }

        // EBCDIC printable range: 0x40-0xFE
        #[test]
        fn ebcdic_printable_validation(v in proptest::collection::vec(any::<u8>(), 0..50)) {
            let valid = v.iter().all(|&b| (0x40..=0xFE).contains(&b));
            let result = validate_ebcdic_printable(&v, 0, usize::MAX);
            prop_assert_eq!(result.is_ok(), valid);
        }

        // Range validation with arbitrary bounds
        #[test]
        fn range_validation(val in any::<i32>(), lo in any::<i32>(), hi in any::<i32>()) {
            prop_assume!(lo <= hi);
            let expected = val >= lo && val < hi;
            prop_assert_eq!(validate_range(val, lo..hi).is_ok(), expected);
        }
        #[test]
        fn even_hex_errors_match_content_and_length(input in prop::collection::vec(any::<u8>(), 0..64), lo in 0usize..64, hi in 0usize..64) {
            let (min, max) = (lo.min(hi), lo.max(hi));
            let expected = if !input.iter().all(u8::is_ascii_hexdigit) {
                Err(Error::Invalid)
            } else if input.len() < min || input.len() > max || !input.len().is_multiple_of(2) {
                Err(Error::InvalidValueLength)
            } else { Ok(input.len()) };
            prop_assert_eq!(validate_hex_even(&input, min, max), expected);
        }

    }
}
