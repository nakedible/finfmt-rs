use std::ops::RangeBounds;

#[cfg(all(not(debug_assertions), feature = "no-panic"))]
use no_panic::no_panic;

use crate::Error;
use crate::primitive::ebcdic::encode_ebcdic_1142_char;
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
    let input = input.as_ref();
    if !input.len().is_multiple_of(2) {
        cold_path();
        return Err(Error::InvalidValueLength);
    }
    validate_bytes(input, minlen, maxlen, pred)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_numeric(input: impl AsRef<[u8]>, minlen: usize, maxlen: usize) -> Result<usize, Error> {
    validate_bytes(input, minlen, maxlen, u8::is_ascii_digit)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn split_signed_input(input: &[u8]) -> Result<(bool, &[u8]), Error> {
    let Some((&first, rest)) = input.split_first() else {
        cold_path();
        return Err(Error::Invalid);
    };
    let (negative, digits) = match first {
        b'-' => (true, rest),
        b'+' => {
            cold_path();
            return Err(Error::Invalid);
        }
        _ => (false, input),
    };
    if digits.is_empty() {
        cold_path();
        return Err(Error::Invalid);
    }
    Ok((negative, digits))
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn parse_signed_decimal(input: &[u8], max_digits: usize) -> Result<(bool, &[u8]), Error> {
    let (negative, digits) = split_signed_input(input)?;
    validate_numeric(digits, 1, max_digits)?;
    Ok((negative, digits))
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub(crate) fn parse_scaled_decimal(input: &[u8], scale: usize, signed: bool) -> Result<(bool, usize, usize, usize, Option<usize>), Error> {
    let Some((&first, rest)) = input.split_first() else {
        cold_path();
        return Err(Error::Invalid);
    };
    let (negative, input) = match first {
        b'-' if signed => (true, rest),
        b'-' => {
            cold_path();
            return Err(Error::Invalid);
        }
        b'+' => {
            cold_path();
            return Err(Error::Invalid);
        }
        _ => (false, input),
    };
    let mut int_digits = 0usize;
    let mut frac_digits = 0usize;
    let mut seen_dot = false;
    let mut first_nonzero = None;
    let mut digit_index = 0usize;
    for &byte in input {
        match byte {
            b'0'..=b'9' => {
                if byte != b'0' && first_nonzero.is_none() {
                    first_nonzero = Some(digit_index);
                }
                if seen_dot {
                    frac_digits += 1;
                } else {
                    int_digits += 1;
                }
                digit_index += 1;
            }
            b'.' if !seen_dot => seen_dot = true,
            _ => {
                cold_path();
                return Err(Error::Invalid);
            }
        }
    }
    if int_digits == 0 || (seen_dot && frac_digits == 0) || frac_digits > scale {
        cold_path();
        return Err(Error::Invalid);
    }
    let total_digits = int_digits.checked_add(scale).ok_or_else(|| {
        cold_path();
        Error::Invalid
    })?;
    Ok((negative, int_digits, frac_digits, total_digits, first_nonzero))
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_decimal_implied(input: &[u8], scale: usize, max_digits: usize, signed: bool) -> Result<usize, Error> {
    let (negative, _int_digits, _frac_digits, total_digits, first_nonzero) = parse_scaled_decimal(input, scale, signed)?;
    let digits_len = match first_nonzero {
        Some(first_nonzero) => total_digits - first_nonzero,
        None => 1,
    };
    if digits_len > max_digits {
        cold_path();
        return Err(Error::InvalidValueLength);
    }
    Ok(digits_len + usize::from(negative && first_nonzero.is_some()))
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

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_bcdz(input: impl AsRef<[u8]>, minlen: usize, maxlen: usize) -> Result<usize, Error> {
    validate_bytes(input, minlen, maxlen, |b| matches!(b, b'0'..=b'?'))
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_track2(input: impl AsRef<[u8]>, minlen: usize, maxlen: usize) -> Result<usize, Error> {
    validate_bytes(input, minlen, maxlen, |b| matches!(b, b'0'..=b'9' | b'='))
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_bcd_bytes(input: &[u8], minlen: usize, maxlen: usize) -> Result<usize, Error> {
    validate_bytes(input, minlen, maxlen, |b| (b >> 4) <= 9 && (b & 0x0F) <= 9)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_binary(input: &[u8], minlen: usize, maxlen: usize) -> Result<usize, Error> {
    validate_bytes(input, minlen, maxlen, |_| true)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_iso8859_1_str(input: &str, minlen: usize, maxlen: usize) -> Result<usize, Error> {
    validate_chars(input, minlen, maxlen, |c| (c as u32) <= 0xFF)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_ebcdic_1142_text(input: &[u8], minlen: usize, maxlen: usize) -> Result<usize, Error> {
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
    let mut count = 0usize;
    for ch in text.chars() {
        if encode_ebcdic_1142_char(ch).is_none() {
            cold_path();
            return Err(Error::Invalid);
        }
        count += 1;
    }
    if count < minlen || count > maxlen {
        cold_path();
        return Err(Error::InvalidValueLength);
    }
    Ok(count)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_ebcdic_printable(input: &[u8], minlen: usize, maxlen: usize) -> Result<usize, Error> {
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
    fn test_validate_signed_decimal_and_implied() {
        assert_eq!(split_signed_input(b"12"), Ok((false, &b"12"[..])));
        assert_eq!(split_signed_input(b"-12"), Ok((true, &b"12"[..])));
        assert_eq!(split_signed_input(b"+12"), Err(Error::Invalid));
        assert_eq!(split_signed_input(b"-"), Err(Error::Invalid));

        assert_eq!(parse_signed_decimal(b"12", 2), Ok((false, &b"12"[..])));
        assert_eq!(parse_signed_decimal(b"-12", 2), Ok((true, &b"12"[..])));
        assert_eq!(parse_signed_decimal(b"-123", 2), Err(Error::InvalidValueLength));
        assert_eq!(parse_signed_decimal(b"+12", 2), Err(Error::Invalid));

        assert_eq!(validate_decimal_implied(b"123.45", 2, 5, false), Ok(5));
        assert_eq!(validate_decimal_implied(b"-0.05", 2, 5, true), Ok(2));
        assert_eq!(validate_decimal_implied(b"1234.56", 2, 5, false), Err(Error::InvalidValueLength));
        assert_eq!(validate_decimal_implied(b".5", 2, 5, false), Err(Error::Invalid));
        assert_eq!(validate_decimal_implied(b"1.", 2, 5, false), Err(Error::Invalid));
        assert_eq!(validate_decimal_implied(b"+1", 2, 5, true), Err(Error::Invalid));
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
        assert_eq!(validate_track2("1234567890=", 0, 99), Ok(11));
        assert_eq!(validate_track2("", 0, 99), Ok(0));
        assert_eq!(validate_track2("1234D", 0, 99), Err(Error::Invalid));
        assert_eq!(validate_track2("12?", 0, 99), Err(Error::Invalid));
        assert_eq!(validate_track2("123", 4, 5), Err(Error::InvalidValueLength));
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
        assert_eq!(validate_binary(b"\x00\xFF\x80\x7F", 0, 99), Ok(4)); // all bytes valid
        assert_eq!(validate_binary(b"", 0, 99), Ok(0));
        assert_eq!(validate_binary(b"hello", 10, 20), Err(Error::InvalidValueLength));
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
        assert_eq!(validate_ebcdic_1142_text(&[0xFF], 0, 99), Err(Error::Invalid));
    }

    #[test]
    fn test_validate_binary_alias_for_iso8859_1_bytes() {
        assert_eq!(validate_binary(b"\x00\x7F\x80\xFF", 0, 99), Ok(4));
        assert_eq!(validate_binary(b"", 0, 99), Ok(0));
        assert_eq!(validate_binary(b"hello", 10, 20), Err(Error::InvalidValueLength));
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
            let result = validate_track2(&v, 0, usize::MAX);
            prop_assert_eq!(result.is_ok(), valid);
        }

        // Binary accepts everything
        #[test]
        fn binary_accepts_all(v in proptest::collection::vec(any::<u8>(), 0..100)) {
            prop_assert_eq!(validate_binary(&v, 0, usize::MAX), Ok(v.len()));
        }

        // ISO-8859-1 str: char count vs byte count
        #[test]
        fn iso8859_1_char_counting(v in proptest::collection::vec(0u8..=255, 0..100)) {
            let s: String = v.iter().map(|&b| b as char).collect();
            prop_assert_eq!(validate_iso8859_1_str(&s, 0, usize::MAX), Ok(s.chars().count()));
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
    }
}
