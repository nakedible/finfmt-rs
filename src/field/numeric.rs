use core::marker::PhantomData;
use core::mem::size_of;

use crate::primitive::bytes::{copy_bytes, take_bytes};
use crate::primitive::decimal::{
    decode_decimal_implied_digits, decode_decimal_packed_fixed, decode_decimal_packed_signed_fixed, decode_ebcdic_zoned_decimal,
    decode_negative_prefix, decode_sign, encode_decimal_implied, encode_decimal_packed_digits, encode_decimal_packed_fixed,
    encode_decimal_packed_signed_fixed, encode_ebcdic_zoned_decimal, encode_ebcdic_zoned_digits, encode_negative_prefix, encode_sign,
    encoded_decimal_implied_len, packed_decimal_max_digits, parse_signed_decimal, prepend_minus, split_signed_input,
};
use crate::primitive::int::{
    decode_binary_i64_be_fixed, decode_binary_u64_be_fixed, decode_nibble_int_fixed, decode_signed_magnitude_i64,
    encode_binary_i64_be_fixed, encode_binary_u64_be_fixed, encode_nibble_int_fixed, validate_binary_i64_be_fixed,
    validate_nibble_int_fixed,
};
use crate::primitive::nibble::NibbleAlphabet;
use crate::primitive::validation::{validate_byte_length, validate_numeric};
use crate::utils::cold_path;
use crate::{Error, ScalarFmt};

/// Prefix a nonnegative magnitude with one of two distinct sign bytes; equal
/// sign bytes are a composition mistake, asserted in debug builds. Typed numeric methods
/// delegate the magnitude to the inner numeric codec.
pub struct SignPrefix<F, const POS: u8 = b'C', const NEG: u8 = b'D'>(PhantomData<F>);
/// Prefix negative magnitudes only. The inner format represents a nonnegative
/// magnitude, and `NEG` must never start one of its encodings. Arbitrary binary
/// formats may violate this; use an explicit `SignPrefix` for those formats.
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
    fn max_digits() -> usize;
    fn encode_digits(output: &mut &mut [u8], digits: &[u8], negative: bool) -> Result<(), Error>;
    const SIGNED: bool;
}

impl<const N: usize> FixedDecimalCodec for FixedComp3<N> {
    const WIRE_LEN: usize = N;
    #[inline(always)]
    fn max_digits() -> usize {
        packed_decimal_max_digits(N)
    }
    const SIGNED: bool = false;
    #[inline(always)]
    fn encode_digits(output: &mut &mut [u8], digits: &[u8], negative: bool) -> Result<(), Error> {
        encode_decimal_packed_digits(output, digits, negative, false, N)
    }
}

impl<const N: usize> FixedDecimalCodec for FixedSignedComp3<N> {
    const WIRE_LEN: usize = N;
    #[inline(always)]
    fn max_digits() -> usize {
        packed_decimal_max_digits(N)
    }
    const SIGNED: bool = true;
    #[inline(always)]
    fn encode_digits(output: &mut &mut [u8], digits: &[u8], negative: bool) -> Result<(), Error> {
        encode_decimal_packed_digits(output, digits, negative, true, N)
    }
}

impl<const N: usize> FixedDecimalCodec for FixedSignedZonedEbcdic<N> {
    const WIRE_LEN: usize = N;
    #[inline(always)]
    fn max_digits() -> usize {
        debug_assert!(N != 0, "zoned decimal width must be nonzero");
        N
    }
    const SIGNED: bool = true;
    #[inline(always)]
    fn encode_digits(output: &mut &mut [u8], digits: &[u8], negative: bool) -> Result<(), Error> {
        encode_ebcdic_zoned_digits(output, digits, negative, N)
    }
}

impl<F, const POS: u8, const NEG: u8> SignPrefix<F, POS, NEG> {
    #[inline(always)]
    fn assert_distinct_signs() {
        debug_assert!(POS != NEG, "SignPrefix needs distinct sign bytes");
    }
}

impl<F: ScalarFmt, const POS: u8, const NEG: u8> ScalarFmt for SignPrefix<F, POS, NEG> {
    #[inline(always)]
    fn encoded_len(input: &[u8]) -> Result<usize, Error> {
        Self::assert_distinct_signs();
        let (_, digits) = split_signed_input(input)?;
        F::encoded_len(digits)?.checked_add(1).ok_or_else(|| {
            cold_path();
            Error::Invalid
        })
    }

    #[inline(always)]
    fn encode(output: &mut &mut [u8], scratch: &mut [u8], input: &[u8]) -> Result<(), Error> {
        Self::assert_distinct_signs();
        let (negative, digits) = split_signed_input(input)?;
        encode_sign(output, negative, POS, NEG)?;
        F::encode(output, scratch, digits)
    }

    #[inline(always)]
    fn decode<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<&'a [u8], Error> {
        Self::assert_distinct_signs();
        let negative = decode_sign(input, POS, NEG)?;
        let digits = F::decode(input, scratch)?;
        if !negative {
            return Ok(digits);
        }
        prepend_minus(scratch, digits).map(|buf| &*buf)
    }

    #[inline(always)]
    fn encoded_len_u64(input: u64) -> Result<usize, Error> {
        Self::assert_distinct_signs();
        F::encoded_len_u64(input)?.checked_add(1).ok_or_else(|| {
            cold_path();
            Error::Invalid
        })
    }

    #[inline(always)]
    fn encode_u64(output: &mut &mut [u8], scratch: &mut [u8], input: u64) -> Result<(), Error> {
        Self::assert_distinct_signs();
        encode_sign(output, false, POS, NEG)?;
        F::encode_u64(output, scratch, input)
    }

    #[inline(always)]
    fn decode_u64<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<u64, Error> {
        Self::assert_distinct_signs();
        if decode_sign(input, POS, NEG)? {
            cold_path();
            return Err(Error::Invalid);
        }
        F::decode_u64(input, scratch)
    }

    #[inline(always)]
    fn encoded_len_i64(input: i64) -> Result<usize, Error> {
        Self::assert_distinct_signs();
        F::encoded_len_u64(input.unsigned_abs())?.checked_add(1).ok_or_else(|| {
            cold_path();
            Error::Invalid
        })
    }

    #[inline(always)]
    fn encode_i64(output: &mut &mut [u8], scratch: &mut [u8], input: i64) -> Result<(), Error> {
        Self::assert_distinct_signs();
        encode_sign(output, input < 0, POS, NEG)?;
        F::encode_u64(output, scratch, input.unsigned_abs())
    }

    #[inline(always)]
    fn decode_i64<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<i64, Error> {
        Self::assert_distinct_signs();
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

    #[inline(always)]
    fn encode(output: &mut &mut [u8], scratch: &mut [u8], input: &[u8]) -> Result<(), Error> {
        let (negative, digits) = split_signed_input(input)?;
        encode_negative_prefix(output, negative, NEG)?;
        F::encode(output, scratch, digits)
    }

    #[inline(always)]
    fn decode<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<&'a [u8], Error> {
        let negative = decode_negative_prefix(input, NEG);
        let digits = F::decode(input, scratch)?;
        if !negative {
            return Ok(digits);
        }
        prepend_minus(scratch, digits).map(|buf| &*buf)
    }

    #[inline(always)]
    fn encoded_len_u64(input: u64) -> Result<usize, Error> {
        F::encoded_len_u64(input)
    }

    #[inline(always)]
    fn encode_u64(output: &mut &mut [u8], scratch: &mut [u8], input: u64) -> Result<(), Error> {
        F::encode_u64(output, scratch, input)
    }

    #[inline(always)]
    fn decode_u64<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<u64, Error> {
        if decode_negative_prefix(input, NEG) {
            cold_path();
            return Err(Error::Invalid);
        }
        F::decode_u64(input, scratch)
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
    fn encode_i64(output: &mut &mut [u8], scratch: &mut [u8], input: i64) -> Result<(), Error> {
        encode_negative_prefix(output, input < 0, NEG)?;
        F::encode_u64(output, scratch, input.unsigned_abs())
    }

    #[inline(always)]
    fn decode_i64<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<i64, Error> {
        let negative = decode_negative_prefix(input, NEG);
        let magnitude = F::decode_u64(input, scratch)?;
        decode_signed_magnitude_i64(negative, magnitude)
    }
}

impl<F: NibbleAlphabet, const N: usize> ScalarFmt for FixedNibbleInt<F, N> {
    #[inline(always)]
    fn encoded_len(input: &[u8]) -> Result<usize, Error> {
        validate_nibble_int_fixed::<F>(input, N)?;
        Ok(N)
    }

    #[inline(always)]
    fn encode(output: &mut &mut [u8], _scratch: &mut [u8], input: &[u8]) -> Result<(), Error> {
        validate_nibble_int_fixed::<F>(input, N)?;
        copy_bytes(output, input)?;
        Ok(())
    }

    #[inline(always)]
    fn decode<'a>(input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<&'a [u8], Error> {
        let bytes = take_bytes(input, N)?;
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
    fn encode_u64(output: &mut &mut [u8], _scratch: &mut [u8], input: u64) -> Result<(), Error> {
        encode_nibble_int_fixed::<F>(output, input, N)
    }

    #[inline(always)]
    fn decode_u64<'a>(input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<u64, Error> {
        decode_nibble_int_fixed::<F>(input, N)
    }

    #[inline(always)]
    fn encoded_len_i64(input: i64) -> Result<usize, Error> {
        if input < 0 {
            cold_path();
            return Err(Error::Invalid);
        }
        Self::encoded_len_u64(input as u64)
    }

    #[inline(always)]
    fn encode_i64(output: &mut &mut [u8], _scratch: &mut [u8], input: i64) -> Result<(), Error> {
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
        validate_byte_length(input, N, N)?;
        Ok(N)
    }

    #[inline(always)]
    fn encode(output: &mut &mut [u8], _scratch: &mut [u8], input: &[u8]) -> Result<(), Error> {
        validate_byte_length(input, N, N)?;
        copy_bytes(output, input)?;
        Ok(())
    }

    #[inline(always)]
    fn decode<'a>(input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<&'a [u8], Error> {
        take_bytes(input, N)
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
    fn encode_u64(output: &mut &mut [u8], _scratch: &mut [u8], input: u64) -> Result<(), Error> {
        encode_binary_u64_be_fixed(output, input, N)
    }

    #[inline(always)]
    fn decode_u64<'a>(input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<u64, Error> {
        decode_binary_u64_be_fixed(input, N)
    }

    #[inline(always)]
    fn encoded_len_i64(input: i64) -> Result<usize, Error> {
        if input < 0 {
            cold_path();
            return Err(Error::Invalid);
        }
        Self::encoded_len_u64(input as u64)
    }

    #[inline(always)]
    fn encode_i64(output: &mut &mut [u8], _scratch: &mut [u8], input: i64) -> Result<(), Error> {
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
        validate_byte_length(input, N, N)?;
        Ok(N)
    }

    #[inline(always)]
    fn encode(output: &mut &mut [u8], _scratch: &mut [u8], input: &[u8]) -> Result<(), Error> {
        validate_byte_length(input, N, N)?;
        copy_bytes(output, input)?;
        Ok(())
    }

    #[inline(always)]
    fn decode<'a>(input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<&'a [u8], Error> {
        take_bytes(input, N)
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
    fn encode_u64(output: &mut &mut [u8], _scratch: &mut [u8], input: u64) -> Result<(), Error> {
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
    fn encoded_len_i64(input: i64) -> Result<usize, Error> {
        validate_binary_i64_be_fixed(input, N)?;
        Ok(N)
    }

    #[inline(always)]
    fn encode_i64(output: &mut &mut [u8], _scratch: &mut [u8], input: i64) -> Result<(), Error> {
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
        validate_numeric(input, 1, packed_decimal_max_digits(N))?;
        Ok(N)
    }

    #[inline(always)]
    fn encode(output: &mut &mut [u8], _scratch: &mut [u8], input: &[u8]) -> Result<(), Error> {
        encode_decimal_packed_fixed(output, input, N)
    }

    #[inline(always)]
    fn decode<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<&'a [u8], Error> {
        decode_decimal_packed_fixed(input, scratch, N).map(|buf| &*buf)
    }
}

impl<const N: usize> ScalarFmt for FixedSignedComp3<N> {
    #[inline(always)]
    fn encoded_len(input: &[u8]) -> Result<usize, Error> {
        parse_signed_decimal(input, packed_decimal_max_digits(N))?;
        Ok(N)
    }

    #[inline(always)]
    fn encode(output: &mut &mut [u8], _scratch: &mut [u8], input: &[u8]) -> Result<(), Error> {
        encode_decimal_packed_signed_fixed(output, input, N)
    }

    #[inline(always)]
    fn decode<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<&'a [u8], Error> {
        decode_decimal_packed_signed_fixed(input, scratch, N).map(|buf| &*buf)
    }
}

impl<const N: usize> ScalarFmt for FixedSignedZonedEbcdic<N> {
    #[inline(always)]
    fn encoded_len(input: &[u8]) -> Result<usize, Error> {
        debug_assert!(N != 0, "zoned decimal width must be nonzero");
        parse_signed_decimal(input, N)?;
        Ok(N)
    }

    #[inline(always)]
    fn encode(output: &mut &mut [u8], _scratch: &mut [u8], input: &[u8]) -> Result<(), Error> {
        encode_ebcdic_zoned_decimal(output, input, N)
    }

    #[inline(always)]
    fn decode<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<&'a [u8], Error> {
        decode_ebcdic_zoned_decimal(input, scratch, N).map(|buf| &*buf)
    }
}

impl<F: FixedDecimalCodec, const SCALE: usize> ScalarFmt for ImpliedDecimal<F, SCALE> {
    #[inline(always)]
    fn encoded_len(input: &[u8]) -> Result<usize, Error> {
        let _ = encoded_decimal_implied_len(input, SCALE, F::max_digits(), F::SIGNED)?;
        Ok(F::WIRE_LEN)
    }

    #[inline(always)]
    fn encode(output: &mut &mut [u8], scratch: &mut [u8], input: &[u8]) -> Result<(), Error> {
        let digits = encode_decimal_implied(&mut &mut *scratch, input, SCALE, F::max_digits(), F::SIGNED)?;
        let (negative, digits) = split_signed_input(digits)?;
        F::encode_digits(output, digits, negative)
    }

    #[inline(always)]
    fn decode<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<&'a [u8], Error> {
        let digits = F::decode(input, scratch)?;
        let (negative, digits) = split_signed_input(digits)?;
        decode_decimal_implied_digits(scratch, digits, negative, SCALE).map(|buf| &*buf)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FixedBinaryBe, FixedComp3, FixedNibbleInt, FixedSignedBinaryBe, FixedSignedComp3, FixedSignedZonedEbcdic, ImpliedDecimal,
        MinusPrefix, SignPrefix,
    };
    use crate::primitive::nibble::{EbcdicHexDigits, LowerHexDigits, UpperHexDigits};
    use crate::{Error, ScalarFmt};

    fn encode_i64<F: ScalarFmt>(value: i64) -> Result<Vec<u8>, Error> {
        let mut output = [0u8; 8];
        let total = output.len();
        let used = {
            let mut out_ptr = output.as_mut_slice();
            let mut scratch = [];
            let scratch_ptr = scratch.as_mut_slice();
            F::encode_i64(&mut out_ptr, scratch_ptr, value)?;
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

    fn encode_padded<F: ScalarFmt>(input: &[u8]) -> Result<Vec<u8>, Error> {
        let mut output = [0u8; 32];
        let total = output.len();
        let used = {
            let mut out_ptr = output.as_mut_slice();
            let mut scratch = [0u8; 32];
            let scratch_ptr = scratch.as_mut_slice();
            F::encode(&mut out_ptr, scratch_ptr, input)?;
            total - out_ptr.len()
        };
        Ok(output[..used].to_vec())
    }

    fn decode_padded<F: ScalarFmt>(input: &[u8]) -> Result<Vec<u8>, Error> {
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
            let scratch_ptr = scratch.as_mut_slice();
            F::encode_u64(&mut out_ptr, scratch_ptr, value)?;
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
    fn wrong_length_values_are_value_length_errors() {
        assert_eq!(FixedBinaryBe::<2>::encoded_len(b"\x01"), Err(Error::InvalidValueLength));
        assert_eq!(encode_padded::<FixedBinaryBe<2>>(b"\x01\x02\x03"), Err(Error::InvalidValueLength));
        assert_eq!(FixedSignedBinaryBe::<2>::encoded_len(b"\x01"), Err(Error::InvalidValueLength));
        assert_eq!(encode_padded::<FixedSignedBinaryBe<2>>(b"\x01"), Err(Error::InvalidValueLength));
        assert_eq!(
            FixedNibbleInt::<UpperHexDigits, 2>::encoded_len(b"F"),
            Err(Error::InvalidValueLength)
        );
        assert_eq!(
            encode_padded::<FixedNibbleInt<UpperHexDigits, 2>>(b"FFF"),
            Err(Error::InvalidValueLength)
        );
        assert_eq!(encode_padded::<FixedNibbleInt<UpperHexDigits, 2>>(b"G"), Err(Error::Invalid));
    }

    #[test]
    fn test_fixed_binary_be_stays_unsigned() {
        assert_eq!(encode_i64::<FixedBinaryBe<1>>(-1), Err(Error::Invalid));
        assert_eq!(decode_i64::<FixedBinaryBe<2>>(b"\xFF\xFF"), Ok(65535));
    }

    #[test]
    fn test_fixed_nibble_int_numeric_api() {
        assert_eq!(encode_u64::<FixedNibbleInt<UpperHexDigits, 2>>(0xFF), Ok(b"FF".to_vec()));
        assert_eq!(encode_u64::<FixedNibbleInt<LowerHexDigits, 2>>(0xAB), Ok(b"ab".to_vec()));
        assert_eq!(encode_u64::<FixedNibbleInt<EbcdicHexDigits, 2>>(0xAF), Ok(b"\xC1\xC6".to_vec()));
        assert_eq!(encode_u64::<FixedNibbleInt<UpperHexDigits, 3>>(0xABC), Ok(b"ABC".to_vec()));
        assert_eq!(decode_u64::<FixedNibbleInt<UpperHexDigits, 2>>(b"FF"), Ok(0xFF));
        assert_eq!(decode_u64::<FixedNibbleInt<LowerHexDigits, 2>>(b"ab"), Ok(0xAB));
        assert_eq!(decode_u64::<FixedNibbleInt<EbcdicHexDigits, 2>>(b"\xC1\xC6"), Ok(0xAF));
        assert_eq!(decode_u64::<FixedNibbleInt<UpperHexDigits, 3>>(b"ABC"), Ok(0xABC));

        assert_eq!(encode_i64::<FixedNibbleInt<UpperHexDigits, 1>>(-1), Err(Error::Invalid));
        assert_eq!(encode_u64::<FixedNibbleInt<UpperHexDigits, 2>>(0x100), Err(Error::Invalid));
    }

    #[test]
    fn test_fixed_signed_zoned_ebcdic_numeric_api() {
        assert_eq!(encode_i64::<FixedSignedZonedEbcdic<2>>(-12), Ok(vec![0xF1, 0xD2]));
        assert_eq!(decode_i64::<FixedSignedZonedEbcdic<2>>(b"\xF1\xC2"), Ok(12));
        assert_eq!(decode_i64::<FixedSignedZonedEbcdic<2>>(b"\xF1\xF2"), Ok(12));
        assert_eq!(encode_i64::<FixedSignedZonedEbcdic<1>>(10), Err(Error::Invalid));
        assert_eq!(decode_u64::<FixedSignedZonedEbcdic<2>>(b"\xF1\xD2"), Err(Error::Invalid));
    }

    #[test]
    fn test_fixed_signed_zoned_ebcdic_byte_api() {
        assert_eq!(encode_padded::<FixedSignedZonedEbcdic<2>>(b"-7"), Ok(vec![0xF0, 0xD7]));
        assert_eq!(encode_padded::<FixedSignedZonedEbcdic<3>>(b"12"), Ok(vec![0xF0, 0xF1, 0xC2]));
        assert_eq!(decode_padded::<FixedSignedZonedEbcdic<2>>(b"\xF0\xD7"), Ok(b"-7".to_vec()));
        assert_eq!(decode_padded::<FixedSignedZonedEbcdic<3>>(b"\xF0\xF0\xC0"), Ok(b"0".to_vec()));
        assert_eq!(encode_padded::<FixedSignedZonedEbcdic<2>>(b"+7"), Err(Error::Invalid));
        assert_eq!(encode_padded::<FixedSignedZonedEbcdic<2>>(b"123"), Err(Error::InvalidValueLength));
        assert_eq!(decode_padded::<FixedSignedZonedEbcdic<2>>(b"\xC1\xC2"), Err(Error::Invalid));
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
        assert_eq!(encode_padded::<FixedComp3<2>>(b"12"), Ok(vec![0x01, 0x2F]));
        assert_eq!(encode_padded::<FixedComp3<2>>(b"123"), Ok(vec![0x12, 0x3F]));
        assert_eq!(decode_padded::<FixedComp3<2>>(b"\x01\x2C"), Ok(b"12".to_vec()));
        assert_eq!(decode_padded::<FixedComp3<2>>(b"\x01\x2F"), Ok(b"12".to_vec()));
        assert_eq!(decode_padded::<FixedComp3<2>>(b"\x00\x0C"), Ok(b"0".to_vec()));
        assert_eq!(encode_padded::<FixedComp3<2>>(b"-7"), Err(Error::Invalid));
        assert_eq!(encode_padded::<FixedComp3<2>>(b"1234"), Err(Error::InvalidValueLength));
        assert_eq!(decode_padded::<FixedComp3<2>>(b"\x01\x2D"), Err(Error::Invalid));
    }

    #[test]
    fn test_fixed_signed_comp3_numeric_api() {
        assert_eq!(encode_i64::<FixedSignedComp3<2>>(-12), Ok(vec![0x01, 0x2D]));
        assert_eq!(decode_i64::<FixedSignedComp3<2>>(b"\x01\x2C"), Ok(12));
        assert_eq!(decode_i64::<FixedSignedComp3<2>>(b"\x01\x2B"), Ok(-12));
        assert_eq!(encode_i64::<FixedSignedComp3<1>>(10), Err(Error::Invalid));
        assert_eq!(decode_u64::<FixedSignedComp3<2>>(b"\x01\x2D"), Err(Error::Invalid));
    }

    #[test]
    fn test_fixed_signed_comp3_byte_api() {
        assert_eq!(encode_padded::<FixedSignedComp3<2>>(b"-7"), Ok(vec![0x00, 0x7D]));
        assert_eq!(encode_padded::<FixedSignedComp3<2>>(b"12"), Ok(vec![0x01, 0x2C]));
        assert_eq!(decode_padded::<FixedSignedComp3<2>>(b"\x00\x7D"), Ok(b"-7".to_vec()));
        assert_eq!(decode_padded::<FixedSignedComp3<2>>(b"\x00\x0D"), Ok(b"0".to_vec()));
        assert_eq!(encode_padded::<FixedSignedComp3<2>>(b"+7"), Err(Error::Invalid));
        assert_eq!(encode_padded::<FixedSignedComp3<2>>(b"1234"), Err(Error::InvalidValueLength));
        assert_eq!(decode_padded::<FixedSignedComp3<2>>(b"\x1A\x2C"), Err(Error::Invalid));
    }

    #[test]
    fn test_implied_decimal_signed_zoned() {
        type F = ImpliedDecimal<FixedSignedZonedEbcdic<5>, 2>;
        assert_eq!(encode_padded::<F>(b"123.45"), Ok(vec![0xF1, 0xF2, 0xF3, 0xF4, 0xC5]));
        assert_eq!(encode_padded::<F>(b"-0.05"), Ok(vec![0xF0, 0xF0, 0xF0, 0xF0, 0xD5]));
        assert_eq!(decode_padded::<F>(b"\xF1\xF2\xF3\xF4\xC5"), Ok(b"123.45".to_vec()));
        assert_eq!(decode_padded::<F>(b"\xF0\xF0\xF1\xF2\xC0"), Ok(b"1.2".to_vec()));
        assert_eq!(encode_padded::<F>(b"1.234"), Err(Error::Invalid));
        assert_eq!(encode_padded::<F>(b"1234.56"), Err(Error::InvalidValueLength));
    }

    #[test]
    fn test_implied_decimal_exact_decode_scratch() {
        type F = ImpliedDecimal<FixedSignedZonedEbcdic<5>, 2>;
        let mut input = &b"\xF0\xF0\xF1\xF2\xC0"[..];
        let mut scratch = [0; 9];
        let mut scratch = scratch.as_mut_slice();
        assert_eq!(F::decode(&mut input, &mut scratch), Ok(&b"1.2"[..]));
        assert!(input.is_empty());
        assert!(scratch.is_empty());
    }

    #[test]
    fn test_implied_decimal_comp3() {
        type F = ImpliedDecimal<FixedComp3<3>, 2>;
        assert_eq!(encode_padded::<F>(b"123.45"), Ok(vec![0x12, 0x34, 0x5F]));
        assert_eq!(decode_padded::<F>(b"\x12\x34\x5C"), Ok(b"123.45".to_vec()));
        assert_eq!(decode_padded::<F>(b"\x12\x34\x5F"), Ok(b"123.45".to_vec()));
        assert_eq!(decode_padded::<F>(b"\x00\x12\x0C"), Ok(b"1.2".to_vec()));
        assert_eq!(encode_padded::<F>(b"-1.23"), Err(Error::Invalid));
    }

    #[test]
    fn test_implied_decimal_signed_comp3() {
        type F = ImpliedDecimal<FixedSignedComp3<3>, 3>;
        assert_eq!(encode_padded::<F>(b"-12.34"), Ok(vec![0x12, 0x34, 0x0D]));
        assert_eq!(encode_padded::<F>(b"12"), Ok(vec![0x12, 0x00, 0x0C]));
        assert_eq!(decode_padded::<F>(b"\x12\x34\x0D"), Ok(b"-12.34".to_vec()));
        assert_eq!(decode_padded::<F>(b"\x12\x00\x0C"), Ok(b"12".to_vec()));
        assert_eq!(encode_padded::<F>(b"-.1"), Err(Error::Invalid));
    }

    pub(super) fn numeric_roundtrip<F: ScalarFmt>(signed: i64, unsigned: u64) {
        let mut output = [0; 64];
        let mut scratch = [0; 128];
        let mut out = output.as_mut_slice();
        let work = scratch.as_mut_slice();
        let result = F::encode_i64(&mut out, work, signed);
        let used = 64 - out.len();
        assert_eq!(result.map(|()| used), F::encoded_len_i64(signed));
        if result.is_ok() {
            let mut input = &output[..used];
            let mut work = scratch.as_mut_slice();
            assert_eq!(F::decode_i64(&mut input, &mut work), Ok(signed));
            assert!(input.is_empty());
        }
        let mut out = output.as_mut_slice();
        let work = scratch.as_mut_slice();
        let result = F::encode_u64(&mut out, work, unsigned);
        let used = 64 - out.len();
        assert_eq!(result.map(|()| used), F::encoded_len_u64(unsigned));
        if result.is_ok() {
            let mut input = &output[..used];
            let mut work = scratch.as_mut_slice();
            assert_eq!(F::decode_u64(&mut input, &mut work), Ok(unsigned));
            assert!(input.is_empty());
            let mut input = &output[..used];
            let mut work = scratch.as_mut_slice();
            assert_eq!(
                F::decode_usize(&mut input, &mut work),
                usize::try_from(unsigned).map_err(|_| Error::Invalid)
            );
        }
    }

    #[test]
    #[cfg(debug_assertions)]
    fn composition_mistakes_are_debug_assertions() {
        fn panics(f: impl FnOnce()) {
            assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).is_err());
        }
        panics(|| {
            let _ = SignPrefix::<FixedBinaryBe<1>, b'X', b'X'>::encoded_len(b"1");
        });
        panics(|| {
            let _ = FixedComp3::<0>::encoded_len(b"0");
        });
        panics(|| {
            let _ = ImpliedDecimal::<FixedSignedComp3<0>, 0>::encoded_len(b"0");
        });
        panics(|| {
            let _ = FixedSignedZonedEbcdic::<0>::encoded_len(b"0");
        });
        panics(|| {
            let _ = FixedSignedBinaryBe::<9>::encoded_len_i64(1);
        });
        panics(|| {
            let _ = crate::primitive::decimal::encode_decimal_ascii_fixed(&mut &mut [0; 4][..], 0, 0);
        });
    }

    #[test]
    fn numeric_length_and_roundtrip_boundaries() {
        for value in [0, 1, 15, 16, 127, 128, 255, 256, i64::MAX, i64::MIN, -1] {
            numeric_roundtrip::<FixedBinaryBe<0>>(value, value as u64);
            numeric_roundtrip::<FixedBinaryBe<1>>(value, value as u64);
            numeric_roundtrip::<FixedBinaryBe<8>>(value, value as u64);
            numeric_roundtrip::<FixedBinaryBe<9>>(value, value as u64);
            numeric_roundtrip::<FixedNibbleInt<UpperHexDigits, 0>>(value, value as u64);
            numeric_roundtrip::<FixedNibbleInt<UpperHexDigits, 1>>(value, value as u64);
            numeric_roundtrip::<FixedNibbleInt<UpperHexDigits, 16>>(value, value as u64);
            numeric_roundtrip::<FixedNibbleInt<UpperHexDigits, 17>>(value, value as u64);
            numeric_roundtrip::<FixedSignedBinaryBe<1>>(value, value as u64);
            numeric_roundtrip::<FixedSignedBinaryBe<8>>(value, value as u64);
            numeric_roundtrip::<SignPrefix<FixedBinaryBe<8>>>(value, value as u64);
            numeric_roundtrip::<MinusPrefix<FixedNibbleInt<LowerHexDigits, 16>>>(value, value as u64);
        }
        assert_eq!(encode_u64::<SignPrefix<FixedBinaryBe<1>>>(1), Ok(vec![b'C', 1]));
        assert_eq!(encode_u64::<MinusPrefix<FixedNibbleInt<UpperHexDigits, 1>>>(15), Ok(b"F".to_vec()));
        assert_eq!(decode_u64::<SignPrefix<FixedBinaryBe<1>>>(&[b'D', 0]), Err(Error::Invalid));
        assert_eq!(
            decode_u64::<MinusPrefix<FixedNibbleInt<UpperHexDigits, 1>>>(b"-0"),
            Err(Error::Invalid)
        );
        assert_eq!(decode_i64::<SignPrefix<FixedBinaryBe<1>>>(&[b'D', 0]), Ok(0));
        assert_eq!(decode_i64::<MinusPrefix<FixedNibbleInt<UpperHexDigits, 1>>>(b"-0"), Ok(0));
    }

    fn scratch_decode<F: ScalarFmt>(wire: &[u8], capacity: usize, expected: Result<&[u8], Error>, consumed: usize) {
        let mut input = wire;
        let mut scratch = vec![0; capacity];
        let mut work = scratch.as_mut_slice();
        assert_eq!(F::decode(&mut input, &mut work), expected);
        if expected.is_ok() {
            assert!(input.is_empty());
            assert_eq!(capacity - work.len(), consumed);
        }
    }

    #[test]
    fn sign_decode_supports_caller_scratch() {
        scratch_decode::<SignPrefix<FixedBinaryBe<1>>>(b"C1", 0, Ok(b"1"), 0);
        scratch_decode::<MinusPrefix<FixedBinaryBe<1>>>(b"1", 0, Ok(b"1"), 0);
        scratch_decode::<SignPrefix<FixedBinaryBe<1>>>(b"D0", 2, Ok(b"-0"), 2);
        scratch_decode::<MinusPrefix<FixedBinaryBe<1>>>(b"-0", 2, Ok(b"-0"), 2);
        scratch_decode::<SignPrefix<FixedBinaryBe<20>>>(b"D12345678901234567890", 21, Ok(b"-12345678901234567890"), 21);
        scratch_decode::<MinusPrefix<FixedBinaryBe<20>>>(b"-12345678901234567890", 21, Ok(b"-12345678901234567890"), 21);
        scratch_decode::<SignPrefix<FixedBinaryBe<20>>>(b"D12345678901234567890", 128, Ok(b"-12345678901234567890"), 21);
        scratch_decode::<MinusPrefix<FixedBinaryBe<20>>>(b"-12345678901234567890", 128, Ok(b"-12345678901234567890"), 21);
        scratch_decode::<SignPrefix<FixedBinaryBe<20>>>(b"D12345678901234567890", 20, Err(Error::BufferOverflow), 0);
        scratch_decode::<MinusPrefix<FixedBinaryBe<20>>>(b"-12345678901234567890", 20, Err(Error::BufferOverflow), 0);
        scratch_decode::<FixedComp3<2>>(&[0x12, 0x3F], 4, Ok(b"123"), 4);
        scratch_decode::<FixedSignedComp3<2>>(&[0x12, 0x3D], 4, Ok(b"-123"), 4);
        scratch_decode::<FixedSignedZonedEbcdic<2>>(&[0xF1, 0xD2], 3, Ok(b"-12"), 3);
        scratch_decode::<FixedComp3<2>>(&[0x12, 0x3F], 3, Err(Error::BufferOverflow), 0);
        scratch_decode::<FixedSignedZonedEbcdic<2>>(&[0xF1, 0xD2], 2, Err(Error::BufferOverflow), 0);
        scratch_decode::<SignPrefix<FixedComp3<2>>>(&[b'D', 0x12, 0x3F], 8, Ok(b"-123"), 8);
        scratch_decode::<MinusPrefix<FixedComp3<2>>>(&[b'-', 0x12, 0x3F], 8, Ok(b"-123"), 8);
        scratch_decode::<SignPrefix<FixedComp3<2>>>(&[b'C', 0x12, 0x3F], 4, Ok(b"123"), 4);
        scratch_decode::<MinusPrefix<FixedComp3<2>>>(&[0x12, 0x3F], 4, Ok(b"123"), 4);
    }

    #[test]
    fn decimal_width_edges_return_errors() {
        scratch_decode::<FixedComp3<{ usize::MAX / 2 + 1 }>>(b"", 0, Err(Error::UnexpectedEof), 0);
        scratch_decode::<FixedSignedComp3<{ usize::MAX / 2 + 1 }>>(b"", 0, Err(Error::UnexpectedEof), 0);
        scratch_decode::<FixedSignedZonedEbcdic<{ usize::MAX }>>(b"", 0, Err(Error::UnexpectedEof), 0);
    }
}

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    use super::*;
    use crate::primitive::nibble::{LowerHexDigits, UpperHexDigits};

    proptest! {
        #[test]
        fn implied_decimal_composition_roundtrips(value in -99_999i64..=99_999) {
            fn check<F: ScalarFmt>(text: &str) {
                let mut wire = [0;16];
                let mut scratch = [0;64];
                let mut out = wire.as_mut_slice();
                F::encode(&mut out, scratch.as_mut_slice(), text.as_bytes()).unwrap();
                let used = 16-out.len();
                assert_eq!(F::encoded_len(text.as_bytes()), Ok(used));
                let mut input = &wire[..used];
                let decoded = F::decode(&mut input, &mut scratch.as_mut_slice()).unwrap();
                let expected = text.trim_end_matches('0').trim_end_matches('.');
                assert_eq!(decoded, expected.as_bytes());
                assert!(input.is_empty());
            }
            let magnitude = value.unsigned_abs();
            let unsigned = format!("{}.{:02}", magnitude/100, magnitude%100);
            let signed = format!("{}{}", if value<0 { "-" } else { "" }, unsigned);
            check::<ImpliedDecimal<FixedComp3<3>,2>>(&unsigned);
            check::<ImpliedDecimal<FixedSignedComp3<3>,2>>(&signed);
            check::<ImpliedDecimal<FixedSignedZonedEbcdic<5>,2>>(&signed);
        }

        #[test]
        fn numeric_lengths_match_encoding(signed: i64, unsigned: u64) {
            super::tests::numeric_roundtrip::<FixedBinaryBe<1>>(signed, unsigned);
            super::tests::numeric_roundtrip::<FixedBinaryBe<7>>(signed, unsigned);
            super::tests::numeric_roundtrip::<FixedBinaryBe<9>>(signed, unsigned);
            super::tests::numeric_roundtrip::<FixedNibbleInt<UpperHexDigits, 1>>(signed, unsigned);
            super::tests::numeric_roundtrip::<FixedNibbleInt<UpperHexDigits, 15>>(signed, unsigned);
            super::tests::numeric_roundtrip::<FixedNibbleInt<UpperHexDigits, 17>>(signed, unsigned);
            super::tests::numeric_roundtrip::<SignPrefix<FixedBinaryBe<8>>>(signed, unsigned);
            super::tests::numeric_roundtrip::<MinusPrefix<FixedNibbleInt<LowerHexDigits, 16>>>(signed, unsigned);
        }
    }
}
