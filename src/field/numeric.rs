use core::marker::PhantomData;
use core::mem::size_of;

use crate::primitive::bytes::{decode_exact_bytes, encode_exact_bytes, validate_exact_length};
use crate::primitive::decimal::{
    MAX_DECIMAL_LEN, decode_decimal_ebcdic_signed_fixed, decode_decimal_implied, decode_decimal_packed_fixed,
    decode_decimal_packed_signed_fixed, decode_negative_prefix, decode_sign, encode_decimal_ebcdic_signed_fixed, encode_decimal_implied,
    encode_decimal_packed_fixed, encode_decimal_packed_signed_fixed, encode_negative_prefix, encode_sign, packed_decimal_max_digits,
    prepend_minus,
};
use crate::primitive::int::{
    decode_binary_i64_be_fixed, decode_binary_u64_be_fixed, decode_nibble_int_fixed, decode_signed_magnitude_i64,
    encode_binary_i64_be_fixed, encode_binary_u64_be_fixed, encode_nibble_int_fixed, validate_binary_i64_be_fixed,
    validate_nibble_int_fixed,
};
use crate::primitive::nibble::NibbleFormat;
use crate::primitive::validation::{parse_signed_decimal, split_signed_input, validate_decimal_implied, validate_numeric};
use crate::utils::{cold_path, take_scratch};
use crate::{Error, ScalarFmt};

pub struct SignPrefix<F, const POS: u8 = b'C', const NEG: u8 = b'D'>(PhantomData<F>);
pub struct MinusPrefix<F, const NEG: u8 = b'-'>(PhantomData<F>);
pub struct FixedNibbleInt<F, const N: usize>(PhantomData<F>);
pub struct FixedBinaryBe<const N: usize>;
pub struct FixedSignedBinaryBe<const N: usize>;
pub struct FixedComp3<const N: usize>;
pub struct FixedSignedComp3<const N: usize>;
pub struct FixedSignedZonedEbcdic<const N: usize>;
pub struct ImpliedDecimal<F, const SCALE: usize>(PhantomData<F>);

trait FixedDecimalCodec: ScalarFmt {
    const WIRE_LEN: usize;
    const MAX_DIGITS: usize;
    const SIGNED: bool;
}

impl<const N: usize> FixedDecimalCodec for FixedComp3<N> {
    const WIRE_LEN: usize = N;
    const MAX_DIGITS: usize = N * 2 - 1;
    const SIGNED: bool = false;
}

impl<const N: usize> FixedDecimalCodec for FixedSignedComp3<N> {
    const WIRE_LEN: usize = N;
    const MAX_DIGITS: usize = N * 2 - 1;
    const SIGNED: bool = true;
}

impl<const N: usize> FixedDecimalCodec for FixedSignedZonedEbcdic<N> {
    const WIRE_LEN: usize = N;
    const MAX_DIGITS: usize = N;
    const SIGNED: bool = true;
}

impl<F: ScalarFmt, const POS: u8, const NEG: u8> ScalarFmt for SignPrefix<F, POS, NEG> {
    #[inline(always)]
    fn encoded_len(input: &[u8]) -> Result<usize, Error> {
        let (_, digits) = split_signed_input(input)?;
        F::encoded_len(digits)?.checked_add(1).ok_or_else(|| {
            cold_path();
            Error::Invalid
        })
    }

    fn encode(output: &mut &mut [u8], scratch: &mut &mut [u8], input: &[u8]) -> Result<(), Error> {
        let (negative, digits) = split_signed_input(input)?;
        encode_sign(output, negative, POS, NEG)?;
        F::encode(output, scratch, digits)
    }

    fn decode<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<&'a [u8], Error> {
        let mut output = take_scratch(scratch, MAX_DECIMAL_LEN)?;
        let negative = decode_sign(input, POS, NEG)?;
        let digits = F::decode(input, scratch)?;
        if !negative {
            return Ok(digits);
        }
        prepend_minus(&mut output, digits).map(|buf| &*buf)
    }

    #[inline(always)]
    fn encoded_len_i64(input: i64) -> Result<usize, Error> {
        F::encoded_len_u64(input.unsigned_abs())?.checked_add(1).ok_or_else(|| {
            cold_path();
            Error::Invalid
        })
    }

    #[inline(always)]
    fn encode_i64(output: &mut &mut [u8], scratch: &mut &mut [u8], input: i64) -> Result<(), Error> {
        encode_sign(output, input < 0, POS, NEG)?;
        F::encode_u64(output, scratch, input.unsigned_abs())
    }

    #[inline(always)]
    fn decode_i64<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<i64, Error> {
        let negative = decode_sign(input, POS, NEG)?;
        let magnitude = F::decode_u64(input, scratch)?;
        decode_signed_magnitude_i64(negative, magnitude)
    }
}

impl<F: ScalarFmt, const NEG: u8> ScalarFmt for MinusPrefix<F, NEG> {
    #[inline(always)]
    fn encoded_len(input: &[u8]) -> Result<usize, Error> {
        let (negative, digits) = split_signed_input(input)?;
        let sign_len = usize::from(negative);
        F::encoded_len(digits)?.checked_add(sign_len).ok_or_else(|| {
            cold_path();
            Error::Invalid
        })
    }

    fn encode(output: &mut &mut [u8], scratch: &mut &mut [u8], input: &[u8]) -> Result<(), Error> {
        let (negative, digits) = split_signed_input(input)?;
        encode_negative_prefix(output, negative, NEG)?;
        F::encode(output, scratch, digits)
    }

    fn decode<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<&'a [u8], Error> {
        let mut output = take_scratch(scratch, MAX_DECIMAL_LEN)?;
        let negative = decode_negative_prefix(input, NEG)?;
        let digits = F::decode(input, scratch)?;
        if !negative {
            return Ok(digits);
        }
        prepend_minus(&mut output, digits).map(|buf| &*buf)
    }

    #[inline(always)]
    fn encoded_len_i64(input: i64) -> Result<usize, Error> {
        let sign_len = usize::from(input < 0);
        F::encoded_len_u64(input.unsigned_abs())?.checked_add(sign_len).ok_or_else(|| {
            cold_path();
            Error::Invalid
        })
    }

    #[inline(always)]
    fn encode_i64(output: &mut &mut [u8], scratch: &mut &mut [u8], input: i64) -> Result<(), Error> {
        encode_negative_prefix(output, input < 0, NEG)?;
        F::encode_u64(output, scratch, input.unsigned_abs())
    }

    #[inline(always)]
    fn decode_i64<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<i64, Error> {
        let negative = decode_negative_prefix(input, NEG)?;
        let magnitude = F::decode_u64(input, scratch)?;
        decode_signed_magnitude_i64(negative, magnitude)
    }
}

impl<F: NibbleFormat, const N: usize> ScalarFmt for FixedNibbleInt<F, N> {
    #[inline(always)]
    fn encoded_len(input: &[u8]) -> Result<usize, Error> {
        validate_nibble_int_fixed::<F>(input, N)?;
        Ok(N)
    }

    fn encode(output: &mut &mut [u8], _scratch: &mut &mut [u8], input: &[u8]) -> Result<(), Error> {
        validate_nibble_int_fixed::<F>(input, N)?;
        encode_exact_bytes(output, input, N)
    }

    fn decode<'a>(input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<&'a [u8], Error> {
        let bytes = decode_exact_bytes(input, N)?;
        validate_nibble_int_fixed::<F>(bytes, N)?;
        Ok(bytes)
    }

    #[inline(always)]
    fn encoded_len_u64(input: u64) -> Result<usize, Error> {
        if N < size_of::<u64>() * 2 && input >> (N * 4) != 0 {
            cold_path();
            return Err(Error::Invalid);
        }
        Ok(N)
    }

    #[inline(always)]
    fn encode_u64(output: &mut &mut [u8], _scratch: &mut &mut [u8], input: u64) -> Result<(), Error> {
        encode_nibble_int_fixed::<F>(output, input, N)
    }

    #[inline(always)]
    fn decode_u64<'a>(input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<u64, Error> {
        decode_nibble_int_fixed::<F>(input, N)
    }

    #[inline(always)]
    fn encoded_len_usize(input: usize) -> Result<usize, Error> {
        Self::encoded_len_u64(input as u64)
    }

    #[inline(always)]
    fn encode_usize(output: &mut &mut [u8], _scratch: &mut &mut [u8], input: usize) -> Result<(), Error> {
        Self::encode_u64(output, _scratch, input as u64)
    }

    #[inline(always)]
    fn decode_usize<'a>(input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<usize, Error> {
        let value = Self::decode_u64(input, _scratch)?;
        usize::try_from(value).map_err(|_| {
            cold_path();
            Error::Invalid
        })
    }

    #[inline(always)]
    fn encoded_len_i64(input: i64) -> Result<usize, Error> {
        if input < 0 {
            cold_path();
            return Err(Error::Invalid);
        }
        Ok(N)
    }

    #[inline(always)]
    fn encode_i64(output: &mut &mut [u8], _scratch: &mut &mut [u8], input: i64) -> Result<(), Error> {
        if input < 0 {
            cold_path();
            return Err(Error::Invalid);
        }
        Self::encode_u64(output, _scratch, input as u64)
    }

    #[inline(always)]
    fn decode_i64<'a>(input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<i64, Error> {
        let value = Self::decode_u64(input, _scratch)?;
        i64::try_from(value).map_err(|_| {
            cold_path();
            Error::Invalid
        })
    }
}

impl<const N: usize> ScalarFmt for FixedBinaryBe<N> {
    #[inline(always)]
    fn encoded_len(input: &[u8]) -> Result<usize, Error> {
        validate_exact_length(input, N)?;
        Ok(N)
    }

    fn encode(output: &mut &mut [u8], _scratch: &mut &mut [u8], input: &[u8]) -> Result<(), Error> {
        encode_exact_bytes(output, input, N)
    }

    fn decode<'a>(input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<&'a [u8], Error> {
        decode_exact_bytes(input, N)
    }

    #[inline(always)]
    fn encoded_len_u64(input: u64) -> Result<usize, Error> {
        if N < size_of::<u64>() && input >> (N * 8) != 0 {
            cold_path();
            return Err(Error::Invalid);
        }
        Ok(N)
    }

    #[inline(always)]
    fn encode_u64(output: &mut &mut [u8], _scratch: &mut &mut [u8], input: u64) -> Result<(), Error> {
        encode_binary_u64_be_fixed(output, input, N)
    }

    #[inline(always)]
    fn decode_u64<'a>(input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<u64, Error> {
        decode_binary_u64_be_fixed(input, N)
    }

    #[inline(always)]
    fn encoded_len_usize(input: usize) -> Result<usize, Error> {
        Self::encoded_len_u64(input as u64)
    }

    #[inline(always)]
    fn encode_usize(output: &mut &mut [u8], _scratch: &mut &mut [u8], input: usize) -> Result<(), Error> {
        Self::encode_u64(output, _scratch, input as u64)
    }

    #[inline(always)]
    fn decode_usize<'a>(input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<usize, Error> {
        let value = Self::decode_u64(input, _scratch)?;
        usize::try_from(value).map_err(|_| {
            cold_path();
            Error::Invalid
        })
    }

    #[inline(always)]
    fn encoded_len_i64(input: i64) -> Result<usize, Error> {
        if input < 0 {
            cold_path();
            return Err(Error::Invalid);
        }
        Ok(N)
    }

    #[inline(always)]
    fn encode_i64(output: &mut &mut [u8], _scratch: &mut &mut [u8], input: i64) -> Result<(), Error> {
        if input < 0 {
            cold_path();
            return Err(Error::Invalid);
        }
        Self::encode_u64(output, _scratch, input as u64)
    }

    #[inline(always)]
    fn decode_i64<'a>(input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<i64, Error> {
        let value = Self::decode_u64(input, _scratch)?;
        i64::try_from(value).map_err(|_| {
            cold_path();
            Error::Invalid
        })
    }
}

impl<const N: usize> ScalarFmt for FixedSignedBinaryBe<N> {
    #[inline(always)]
    fn encoded_len(input: &[u8]) -> Result<usize, Error> {
        validate_exact_length(input, N)?;
        Ok(N)
    }

    fn encode(output: &mut &mut [u8], _scratch: &mut &mut [u8], input: &[u8]) -> Result<(), Error> {
        encode_exact_bytes(output, input, N)
    }

    fn decode<'a>(input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<&'a [u8], Error> {
        decode_exact_bytes(input, N)
    }

    #[inline(always)]
    fn encoded_len_u64(input: u64) -> Result<usize, Error> {
        let input = i64::try_from(input).map_err(|_| {
            cold_path();
            Error::Invalid
        })?;
        Self::encoded_len_i64(input)
    }

    #[inline(always)]
    fn encode_u64(output: &mut &mut [u8], _scratch: &mut &mut [u8], input: u64) -> Result<(), Error> {
        let input = i64::try_from(input).map_err(|_| {
            cold_path();
            Error::Invalid
        })?;
        Self::encode_i64(output, _scratch, input)
    }

    #[inline(always)]
    fn decode_u64<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<u64, Error> {
        let value = Self::decode_i64(input, scratch)?;
        u64::try_from(value).map_err(|_| {
            cold_path();
            Error::Invalid
        })
    }

    #[inline(always)]
    fn encoded_len_usize(input: usize) -> Result<usize, Error> {
        Self::encoded_len_u64(input as u64)
    }

    #[inline(always)]
    fn encode_usize(output: &mut &mut [u8], scratch: &mut &mut [u8], input: usize) -> Result<(), Error> {
        Self::encode_u64(output, scratch, input as u64)
    }

    #[inline(always)]
    fn decode_usize<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<usize, Error> {
        let value = Self::decode_u64(input, scratch)?;
        usize::try_from(value).map_err(|_| {
            cold_path();
            Error::Invalid
        })
    }

    #[inline(always)]
    fn encoded_len_i64(input: i64) -> Result<usize, Error> {
        validate_binary_i64_be_fixed(input, N)?;
        Ok(N)
    }

    #[inline(always)]
    fn encode_i64(output: &mut &mut [u8], _scratch: &mut &mut [u8], input: i64) -> Result<(), Error> {
        encode_binary_i64_be_fixed(output, input, N)
    }

    #[inline(always)]
    fn decode_i64<'a>(input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<i64, Error> {
        decode_binary_i64_be_fixed(input, N)
    }
}

impl<const N: usize> ScalarFmt for FixedComp3<N> {
    #[inline(always)]
    fn encoded_len(input: &[u8]) -> Result<usize, Error> {
        validate_numeric(input, 1, packed_decimal_max_digits(N)?)?;
        Ok(N)
    }

    fn encode(output: &mut &mut [u8], _scratch: &mut &mut [u8], input: &[u8]) -> Result<(), Error> {
        encode_decimal_packed_fixed(output, input, N)
    }

    fn decode<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<&'a [u8], Error> {
        let mut output = take_scratch(scratch, N * 2)?;
        decode_decimal_packed_fixed(input, &mut output, N).map(|buf| &*buf)
    }
}

impl<const N: usize> ScalarFmt for FixedSignedComp3<N> {
    #[inline(always)]
    fn encoded_len(input: &[u8]) -> Result<usize, Error> {
        parse_signed_decimal(input, packed_decimal_max_digits(N)?)?;
        Ok(N)
    }

    fn encode(output: &mut &mut [u8], _scratch: &mut &mut [u8], input: &[u8]) -> Result<(), Error> {
        encode_decimal_packed_signed_fixed(output, input, N)
    }

    fn decode<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<&'a [u8], Error> {
        let mut output = take_scratch(scratch, N * 2)?;
        decode_decimal_packed_signed_fixed(input, &mut output, N).map(|buf| &*buf)
    }
}

impl<const N: usize> ScalarFmt for FixedSignedZonedEbcdic<N> {
    #[inline(always)]
    fn encoded_len(input: &[u8]) -> Result<usize, Error> {
        parse_signed_decimal(input, N)?;
        Ok(N)
    }

    fn encode(output: &mut &mut [u8], _scratch: &mut &mut [u8], input: &[u8]) -> Result<(), Error> {
        encode_decimal_ebcdic_signed_fixed(output, input, N)
    }

    fn decode<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<&'a [u8], Error> {
        let mut output = take_scratch(scratch, N + 1)?;
        decode_decimal_ebcdic_signed_fixed(input, &mut output, N).map(|buf| &*buf)
    }
}

impl<F: FixedDecimalCodec, const SCALE: usize> ScalarFmt for ImpliedDecimal<F, SCALE> {
    #[inline(always)]
    fn encoded_len(input: &[u8]) -> Result<usize, Error> {
        let _ = validate_decimal_implied(input, SCALE, F::MAX_DIGITS, F::SIGNED)?;
        Ok(F::WIRE_LEN)
    }

    fn encode(output: &mut &mut [u8], scratch: &mut &mut [u8], input: &[u8]) -> Result<(), Error> {
        let transformed_len = validate_decimal_implied(input, SCALE, F::MAX_DIGITS, F::SIGNED)?;
        let transformed = take_scratch(scratch, transformed_len)?;
        let mut transformed = transformed;
        let digits = encode_decimal_implied(&mut transformed, input, SCALE, F::MAX_DIGITS, F::SIGNED)?;
        F::encode(output, scratch, digits)
    }

    fn decode<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<&'a [u8], Error> {
        let digits = F::decode(input, scratch)?;
        let out_len = digits.len().checked_add(SCALE).and_then(|v| v.checked_add(2)).ok_or_else(|| {
            cold_path();
            Error::BufferOverflow
        })?;
        let output = take_scratch(scratch, out_len)?;
        let mut output = output;
        decode_decimal_implied(&mut output, digits, SCALE).map(|buf| &*buf)
    }
}

#[cfg(test)]
mod tests {
    use super::{FixedBinaryBe, FixedComp3, FixedNibbleInt, FixedSignedBinaryBe, FixedSignedComp3, FixedSignedZonedEbcdic, ImpliedDecimal};
    use crate::primitive::nibble::{HexEbcdic, HexLower, HexUpper};
    use crate::{Error, ScalarFmt};

    fn encode_i64<F: ScalarFmt>(value: i64) -> Result<Vec<u8>, Error> {
        let mut output = [0u8; 8];
        let total = output.len();
        let used = {
            let mut out_ptr = output.as_mut_slice();
            let mut scratch = [];
            let mut scratch_ptr = scratch.as_mut_slice();
            F::encode_i64(&mut out_ptr, &mut scratch_ptr, value)?;
            total - out_ptr.len()
        };
        Ok(output[..used].to_vec())
    }

    fn decode_i64<F: ScalarFmt>(input: &[u8]) -> Result<i64, Error> {
        let mut input = input;
        let mut scratch = [0u8; 32];
        let mut scratch_ptr = scratch.as_mut_slice();
        F::decode_i64(&mut input, &mut scratch_ptr)
    }

    fn decode_u64<F: ScalarFmt>(input: &[u8]) -> Result<u64, Error> {
        let mut input = input;
        let mut scratch = [0u8; 32];
        let mut scratch_ptr = scratch.as_mut_slice();
        F::decode_u64(&mut input, &mut scratch_ptr)
    }

    fn encode_bytes<F: ScalarFmt>(input: &[u8]) -> Result<Vec<u8>, Error> {
        let mut output = [0u8; 32];
        let total = output.len();
        let used = {
            let mut out_ptr = output.as_mut_slice();
            let mut scratch = [0u8; 32];
            let mut scratch_ptr = scratch.as_mut_slice();
            F::encode(&mut out_ptr, &mut scratch_ptr, input)?;
            total - out_ptr.len()
        };
        Ok(output[..used].to_vec())
    }

    fn decode_bytes<F: ScalarFmt>(input: &[u8]) -> Result<Vec<u8>, Error> {
        let mut input = input;
        let mut scratch = [0u8; 32];
        let mut scratch_ptr = scratch.as_mut_slice();
        Ok(F::decode(&mut input, &mut scratch_ptr)?.to_vec())
    }

    fn encode_u64<F: ScalarFmt>(value: u64) -> Result<Vec<u8>, Error> {
        let mut output = [0u8; 32];
        let total = output.len();
        let used = {
            let mut out_ptr = output.as_mut_slice();
            let mut scratch = [0u8; 32];
            let mut scratch_ptr = scratch.as_mut_slice();
            F::encode_u64(&mut out_ptr, &mut scratch_ptr, value)?;
            total - out_ptr.len()
        };
        Ok(output[..used].to_vec())
    }

    #[test]
    fn test_fixed_signed_binary_be_numeric_api() {
        assert_eq!(encode_i64::<FixedSignedBinaryBe<1>>(-1), Ok(vec![0xFF]));
        assert_eq!(encode_i64::<FixedSignedBinaryBe<2>>(0x1234), Ok(vec![0x12, 0x34]));
        assert_eq!(decode_i64::<FixedSignedBinaryBe<1>>(b"\x80"), Ok(-128));
        assert_eq!(decode_i64::<FixedSignedBinaryBe<2>>(b"\xFF\xFE"), Ok(-2));
        assert_eq!(encode_i64::<FixedSignedBinaryBe<1>>(128), Err(Error::Invalid));
        assert_eq!(decode_u64::<FixedSignedBinaryBe<1>>(b"\xFF"), Err(Error::Invalid));
    }

    #[test]
    fn test_fixed_binary_be_stays_unsigned() {
        assert_eq!(encode_i64::<FixedBinaryBe<1>>(-1), Err(Error::Invalid));
        assert_eq!(decode_i64::<FixedBinaryBe<2>>(b"\xFF\xFF"), Ok(65535));
    }

    #[test]
    fn test_fixed_nibble_int_numeric_api() {
        assert_eq!(encode_u64::<FixedNibbleInt<HexUpper, 2>>(0xFF), Ok(b"FF".to_vec()));
        assert_eq!(encode_u64::<FixedNibbleInt<HexLower, 2>>(0xAB), Ok(b"ab".to_vec()));
        assert_eq!(encode_u64::<FixedNibbleInt<HexEbcdic, 2>>(0xAF), Ok(b"\xC1\xC6".to_vec()));
        assert_eq!(encode_u64::<FixedNibbleInt<HexUpper, 3>>(0xABC), Ok(b"ABC".to_vec()));
        assert_eq!(decode_u64::<FixedNibbleInt<HexUpper, 2>>(b"FF"), Ok(0xFF));
        assert_eq!(decode_u64::<FixedNibbleInt<HexLower, 2>>(b"ab"), Ok(0xAB));
        assert_eq!(decode_u64::<FixedNibbleInt<HexEbcdic, 2>>(b"\xC1\xC6"), Ok(0xAF));
        assert_eq!(decode_u64::<FixedNibbleInt<HexUpper, 3>>(b"ABC"), Ok(0xABC));

        assert_eq!(encode_i64::<FixedNibbleInt<HexUpper, 1>>(-1), Err(Error::Invalid));
        assert_eq!(encode_u64::<FixedNibbleInt<HexUpper, 2>>(0x100), Err(Error::Invalid));
    }

    #[test]
    fn test_fixed_signed_zoned_ebcdic_numeric_api() {
        assert_eq!(encode_i64::<FixedSignedZonedEbcdic<2>>(-12), Ok(vec![0xF1, 0xD2]));
        assert_eq!(decode_i64::<FixedSignedZonedEbcdic<2>>(b"\xF1\xC2"), Ok(12));
        assert_eq!(decode_i64::<FixedSignedZonedEbcdic<2>>(b"\xF1\xF2"), Ok(12));
        assert_eq!(encode_i64::<FixedSignedZonedEbcdic<1>>(10), Err(Error::InvalidValueLength));
        assert_eq!(decode_u64::<FixedSignedZonedEbcdic<2>>(b"\xF1\xD2"), Err(Error::Invalid));
    }

    #[test]
    fn test_fixed_signed_zoned_ebcdic_byte_api() {
        assert_eq!(encode_bytes::<FixedSignedZonedEbcdic<2>>(b"-7"), Ok(vec![0xF0, 0xD7]));
        assert_eq!(encode_bytes::<FixedSignedZonedEbcdic<3>>(b"12"), Ok(vec![0xF0, 0xF1, 0xC2]));
        assert_eq!(decode_bytes::<FixedSignedZonedEbcdic<2>>(b"\xF0\xD7"), Ok(b"-7".to_vec()));
        assert_eq!(decode_bytes::<FixedSignedZonedEbcdic<3>>(b"\xF0\xF0\xC0"), Ok(b"0".to_vec()));
        assert_eq!(encode_bytes::<FixedSignedZonedEbcdic<2>>(b"+7"), Err(Error::Invalid));
        assert_eq!(encode_bytes::<FixedSignedZonedEbcdic<2>>(b"123"), Err(Error::InvalidValueLength));
        assert_eq!(decode_bytes::<FixedSignedZonedEbcdic<2>>(b"\xC1\xC2"), Err(Error::Invalid));
    }

    #[test]
    fn test_fixed_comp3_numeric_api() {
        assert_eq!(encode_i64::<FixedComp3<2>>(12), Ok(vec![0x01, 0x2F]));
        assert_eq!(decode_i64::<FixedComp3<2>>(b"\x01\x2C"), Ok(12));
        assert_eq!(decode_i64::<FixedComp3<2>>(b"\x01\x2F"), Ok(12));
        assert_eq!(encode_i64::<FixedComp3<1>>(-1), Err(Error::Invalid));
        assert_eq!(decode_u64::<FixedComp3<2>>(b"\x01\x2D"), Err(Error::Invalid));
    }

    #[test]
    fn test_fixed_comp3_byte_api() {
        assert_eq!(encode_bytes::<FixedComp3<2>>(b"12"), Ok(vec![0x01, 0x2F]));
        assert_eq!(encode_bytes::<FixedComp3<2>>(b"123"), Ok(vec![0x12, 0x3F]));
        assert_eq!(decode_bytes::<FixedComp3<2>>(b"\x01\x2C"), Ok(b"12".to_vec()));
        assert_eq!(decode_bytes::<FixedComp3<2>>(b"\x01\x2F"), Ok(b"12".to_vec()));
        assert_eq!(decode_bytes::<FixedComp3<2>>(b"\x00\x0C"), Ok(b"0".to_vec()));
        assert_eq!(encode_bytes::<FixedComp3<2>>(b"-7"), Err(Error::Invalid));
        assert_eq!(encode_bytes::<FixedComp3<2>>(b"1234"), Err(Error::InvalidValueLength));
        assert_eq!(decode_bytes::<FixedComp3<2>>(b"\x01\x2D"), Err(Error::Invalid));
    }

    #[test]
    fn test_fixed_signed_comp3_numeric_api() {
        assert_eq!(encode_i64::<FixedSignedComp3<2>>(-12), Ok(vec![0x01, 0x2D]));
        assert_eq!(decode_i64::<FixedSignedComp3<2>>(b"\x01\x2C"), Ok(12));
        assert_eq!(decode_i64::<FixedSignedComp3<2>>(b"\x01\x2B"), Ok(-12));
        assert_eq!(encode_i64::<FixedSignedComp3<1>>(10), Err(Error::InvalidValueLength));
        assert_eq!(decode_u64::<FixedSignedComp3<2>>(b"\x01\x2D"), Err(Error::Invalid));
    }

    #[test]
    fn test_fixed_signed_comp3_byte_api() {
        assert_eq!(encode_bytes::<FixedSignedComp3<2>>(b"-7"), Ok(vec![0x00, 0x7D]));
        assert_eq!(encode_bytes::<FixedSignedComp3<2>>(b"12"), Ok(vec![0x01, 0x2C]));
        assert_eq!(decode_bytes::<FixedSignedComp3<2>>(b"\x00\x7D"), Ok(b"-7".to_vec()));
        assert_eq!(decode_bytes::<FixedSignedComp3<2>>(b"\x00\x0D"), Ok(b"0".to_vec()));
        assert_eq!(encode_bytes::<FixedSignedComp3<2>>(b"+7"), Err(Error::Invalid));
        assert_eq!(encode_bytes::<FixedSignedComp3<2>>(b"1234"), Err(Error::InvalidValueLength));
        assert_eq!(decode_bytes::<FixedSignedComp3<2>>(b"\x1A\x2C"), Err(Error::Invalid));
    }

    #[test]
    fn test_implied_decimal_signed_zoned() {
        type F = ImpliedDecimal<FixedSignedZonedEbcdic<5>, 2>;
        assert_eq!(encode_bytes::<F>(b"123.45"), Ok(vec![0xF1, 0xF2, 0xF3, 0xF4, 0xC5]));
        assert_eq!(encode_bytes::<F>(b"-0.05"), Ok(vec![0xF0, 0xF0, 0xF0, 0xF0, 0xD5]));
        assert_eq!(decode_bytes::<F>(b"\xF1\xF2\xF3\xF4\xC5"), Ok(b"123.45".to_vec()));
        assert_eq!(decode_bytes::<F>(b"\xF0\xF0\xF1\xF2\xC0"), Ok(b"1.2".to_vec()));
        assert_eq!(encode_bytes::<F>(b"1.234"), Err(Error::Invalid));
        assert_eq!(encode_bytes::<F>(b"1234.56"), Err(Error::InvalidValueLength));
    }

    #[test]
    fn test_implied_decimal_comp3() {
        type F = ImpliedDecimal<FixedComp3<3>, 2>;
        assert_eq!(encode_bytes::<F>(b"123.45"), Ok(vec![0x12, 0x34, 0x5F]));
        assert_eq!(decode_bytes::<F>(b"\x12\x34\x5C"), Ok(b"123.45".to_vec()));
        assert_eq!(decode_bytes::<F>(b"\x12\x34\x5F"), Ok(b"123.45".to_vec()));
        assert_eq!(decode_bytes::<F>(b"\x00\x12\x0C"), Ok(b"1.2".to_vec()));
        assert_eq!(encode_bytes::<F>(b"-1.23"), Err(Error::Invalid));
    }

    #[test]
    fn test_implied_decimal_signed_comp3() {
        type F = ImpliedDecimal<FixedSignedComp3<3>, 3>;
        assert_eq!(encode_bytes::<F>(b"-12.34"), Ok(vec![0x12, 0x34, 0x0D]));
        assert_eq!(encode_bytes::<F>(b"12"), Ok(vec![0x12, 0x00, 0x0C]));
        assert_eq!(decode_bytes::<F>(b"\x12\x34\x0D"), Ok(b"-12.34".to_vec()));
        assert_eq!(decode_bytes::<F>(b"\x12\x00\x0C"), Ok(b"12".to_vec()));
        assert_eq!(encode_bytes::<F>(b"-.1"), Err(Error::Invalid));
    }
}
