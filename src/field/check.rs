use crate::Error;
use crate::primitive::validation::{
    validate_alpha, validate_alphanum, validate_ascii, validate_ascii_printable, validate_bcd_bytes, validate_bcdz, validate_byte_length,
    validate_ebcdic_037_ascii, validate_ebcdic_1142_text, validate_ebcdic_printable, validate_hex, validate_hex_even, validate_lower_hex,
    validate_lower_hex_even, validate_numeric, validate_track2_chars, validate_upper_alpha, validate_upper_alphanum,
    validate_upper_ascii_printable, validate_upper_hex, validate_upper_hex_even,
};

/// Validate a field's value and return its logical length.
///
/// A field runs its check on the value before encoding it, and on the decoded
/// value after decoding it. The built-in checks take `MIN` and `MAX` as an
/// inclusive length range. They count bytes, except `Ebcdic1142Text`, which
/// counts characters. The field's first `Step` and length spec must count in the
/// same unit. Content errors return `Invalid` and take precedence over length
/// errors, which return `InvalidValueLength`. When decoding, fields report
/// both as `Invalid`.
///
/// Checks marked "wire bytes" validate encoded bytes rather than a value; use
/// them inside `DecodeCheck` after the step that produces those bytes.
///
/// A custom check can be one line with
/// [`validate_bytes`](crate::primitive::validation::validate_bytes).
pub trait Check {
    fn validate(input: &[u8]) -> Result<usize, Error>;
}

/// ASCII decimal digits `0`–`9`.
pub struct Numeric<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for Numeric<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_numeric(input, MIN, MAX)
    }
}

/// ASCII letters `A`–`Z` and `a`–`z`.
pub struct Alpha<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for Alpha<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_alpha(input, MIN, MAX)
    }
}

/// ASCII letters and decimal digits.
pub struct Alphanum<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for Alphanum<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_alphanum(input, MIN, MAX)
    }
}

/// Any ASCII byte, 0x00–0x7F, including control characters.
pub struct Ascii<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for Ascii<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_ascii(input, MIN, MAX)
    }
}

/// Printable ASCII, space (0x20) through `~` (0x7E).
pub struct AsciiPrintable<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for AsciiPrintable<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_ascii_printable(input, MIN, MAX)
    }
}

/// ASCII uppercase letters `A`–`Z`.
pub struct UpperAlpha<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for UpperAlpha<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_upper_alpha(input, MIN, MAX)
    }
}

/// ASCII decimal digits and uppercase letters.
pub struct UpperAlphanum<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for UpperAlphanum<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_upper_alphanum(input, MIN, MAX)
    }
}

/// Printable ASCII without lowercase letters.
pub struct UpperAsciiPrintable<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for UpperAsciiPrintable<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_upper_ascii_printable(input, MIN, MAX)
    }
}

/// Hexadecimal digits in either case: `0`–`9`, `A`–`F`, `a`–`f`.
pub struct Hex<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for Hex<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_hex(input, MIN, MAX)
    }
}

/// Uppercase hexadecimal digits: `0`–`9`, `A`–`F`.
pub struct UpperHex<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for UpperHex<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_upper_hex(input, MIN, MAX)
    }
}

/// Lowercase hexadecimal digits: `0`–`9`, `a`–`f`.
pub struct LowerHex<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for LowerHex<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_lower_hex(input, MIN, MAX)
    }
}

/// Hexadecimal digits in either case, with an even length.
pub struct HexEven<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for HexEven<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_hex_even(input, MIN, MAX)
    }
}

/// Uppercase hexadecimal digits, with an even length.
pub struct UpperHexEven<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for UpperHexEven<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_upper_hex_even(input, MIN, MAX)
    }
}

/// Lowercase hexadecimal digits, with an even length.
pub struct LowerHexEven<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for LowerHexEven<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_lower_hex_even(input, MIN, MAX)
    }
}

/// ASCII decimal digits of a value stored as BCD. Validates exactly like
/// [`Numeric`]; the name documents the field's intent.
pub struct Bcd<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for Bcd<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_numeric(input, MIN, MAX)
    }
}

/// Expanded BCD-Z digits, `0` (0x30) through `?` (0x3F): decimal digits plus
/// `:;<=>?`, which stand for the nibbles A–F.
pub struct Bcdz<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for Bcdz<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_bcdz(input, MIN, MAX)
    }
}

/// Decimal digits and `=`. Checks the characters only, not the Track 2
/// layout or separator position.
pub struct Track2<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for Track2<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_track2_chars(input, MIN, MAX)
    }
}

/// Wire bytes: packed BCD, where both nibbles of every byte are 0–9.
pub struct BcdBytes<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for BcdBytes<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_bcd_bytes(input, MIN, MAX)
    }
}

/// Any bytes; checks only the length.
pub struct Binary<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for Binary<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_byte_length(input, MIN, MAX)
    }
}

/// Placeholder for Latin-1 text. Currently accepts any bytes and checks only
/// the length.
pub struct Iso88591<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for Iso88591<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_byte_length(input, MIN, MAX)
    }
}

/// Wire bytes: CP037 bytes that map to ASCII, including controls. Use
/// `DecodeCheck<Ebcdic037, Ebcdic037Ascii<MIN, MAX>>` to reject, rather than
/// replace, characters outside ASCII when decoding.
pub struct Ebcdic037Ascii<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for Ebcdic037Ascii<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_ebcdic_037_ascii(input, MIN, MAX)
    }
}

/// Wire bytes: EBCDIC 0x40–0xFE, the non-control range of CP037 and CP1142.
/// Includes space, non-breaking space and soft hyphen; does not imply that
/// the characters exist in ASCII.
pub struct EbcdicPrintable<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for EbcdicPrintable<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_ebcdic_printable(input, MIN, MAX)
    }
}

/// UTF-8 text whose characters all exist in IBM1142. `MIN` and `MAX` count
/// characters, not bytes.
pub struct Ebcdic1142Text<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for Ebcdic1142Text<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_ebcdic_1142_text(input, MIN, MAX)
    }
}
