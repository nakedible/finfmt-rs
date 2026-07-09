use crate::Error;
use crate::primitive::validation::{
    validate_alpha, validate_alphanum, validate_ascii, validate_ascii_printable, validate_bcd_bytes, validate_bcdz, validate_binary,
    validate_ebcdic_1142_text, validate_ebcdic_printable, validate_hex, validate_hex_even, validate_hex_lower, validate_hex_lower_even,
    validate_hex_upper, validate_hex_upper_even, validate_numeric, validate_track2, validate_upper_alpha, validate_upper_alphanum,
    validate_upper_ascii_printable,
};

pub trait Check {
    fn validate(input: &[u8]) -> Result<usize, Error>;
}

pub struct Numeric<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for Numeric<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_numeric(input, MIN, MAX)
    }
}

pub struct Alpha<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for Alpha<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_alpha(input, MIN, MAX)
    }
}

pub struct Alphanum<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for Alphanum<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_alphanum(input, MIN, MAX)
    }
}

pub struct Ascii<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for Ascii<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_ascii(input, MIN, MAX)
    }
}

pub struct AsciiPrintable<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for AsciiPrintable<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_ascii_printable(input, MIN, MAX)
    }
}

pub struct UpperAlpha<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for UpperAlpha<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_upper_alpha(input, MIN, MAX)
    }
}

pub struct UpperAlphanum<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for UpperAlphanum<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_upper_alphanum(input, MIN, MAX)
    }
}

pub struct UpperAsciiPrintable<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for UpperAsciiPrintable<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_upper_ascii_printable(input, MIN, MAX)
    }
}

pub struct Hex<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for Hex<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_hex(input, MIN, MAX)
    }
}

pub struct HexUpper<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for HexUpper<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_hex_upper(input, MIN, MAX)
    }
}

pub struct HexLower<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for HexLower<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_hex_lower(input, MIN, MAX)
    }
}

pub struct HexEven<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for HexEven<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_hex_even(input, MIN, MAX)
    }
}

pub struct HexUpperEven<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for HexUpperEven<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_hex_upper_even(input, MIN, MAX)
    }
}

pub struct HexLowerEven<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for HexLowerEven<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_hex_lower_even(input, MIN, MAX)
    }
}

pub struct Bcd<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for Bcd<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_numeric(input, MIN, MAX)
    }
}

pub struct Bcdz<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for Bcdz<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_bcdz(input, MIN, MAX)
    }
}

pub struct Track2<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for Track2<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_track2(input, MIN, MAX)
    }
}

pub struct BcdBytes<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for BcdBytes<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_bcd_bytes(input, MIN, MAX)
    }
}

pub struct Binary<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for Binary<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_binary(input, MIN, MAX)
    }
}

pub struct Iso88591<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for Iso88591<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_binary(input, MIN, MAX)
    }
}

pub struct EbcdicPrintable<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for EbcdicPrintable<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_ebcdic_printable(input, MIN, MAX)
    }
}

pub struct Ebcdic1142Text<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for Ebcdic1142Text<MIN, MAX> {
    #[inline(always)]
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_ebcdic_1142_text(input, MIN, MAX)
    }
}
