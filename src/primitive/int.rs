use core::mem::size_of;

#[cfg(all(not(debug_assertions), feature = "no-panic"))]
use no_panic::no_panic;

use crate::Error;
use crate::primitive::bytes::{all_bytes_eq, validate_exact_length};
use crate::primitive::nibble::{NibbleFormat, pack_nibbles, unpack_padded_nibbles, validate_nibbles};
use crate::utils::cold_path;

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn encode_be_bytes(output: &mut &mut [u8], bytes: &[u8], len: usize, fill: u8) -> Result<(), Error> {
    let buf = output.split_off_mut(..len).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    let prefix_len = len.saturating_sub(bytes.len());
    let tail_len = len - prefix_len;
    if tail_len < bytes.len() && !all_bytes_eq(&bytes[..bytes.len() - tail_len], fill) {
        cold_path();
        return Err(Error::Invalid);
    }
    if prefix_len != 0 {
        buf[..prefix_len].fill(fill);
    }
    buf[prefix_len..].copy_from_slice(&bytes[bytes.len() - tail_len..]);
    Ok(())
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn extend_be_bytes<const N: usize>(input: &mut &[u8], len: usize, fill: u8) -> Result<[u8; N], Error> {
    let bytes = input.split_off(..len).ok_or_else(|| {
        cold_path();
        Error::UnexpectedEof
    })?;
    let prefix_len = len.saturating_sub(N);
    if prefix_len != 0 && !all_bytes_eq(&bytes[..prefix_len], fill) {
        cold_path();
        return Err(Error::Invalid);
    }
    let tail = &bytes[prefix_len..];
    let mut extended = [fill; N];
    let start = extended.len() - tail.len();
    extended[start..].copy_from_slice(tail);
    Ok(extended)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_nibble_int_fixed<F: NibbleFormat>(input: &[u8], len: usize) -> Result<(), Error> {
    validate_exact_length(input, len)?;
    validate_nibbles(input, &F::TABLE)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn encode_nibble_int_fixed<F: NibbleFormat>(output: &mut &mut [u8], value: u64, len: usize) -> Result<(), Error> {
    const WORD_BYTES: usize = size_of::<u64>();
    const WORD_DIGITS: usize = WORD_BYTES * 2;

    let buf = output.split_off_mut(..len).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    let prefix_len = len.saturating_sub(WORD_DIGITS);
    let tail_len = len - prefix_len;
    if prefix_len != 0 {
        buf[..prefix_len].fill(F::DIGITS[0]);
    }
    let packed_len = tail_len.div_ceil(2);
    let mut packed = [0u8; WORD_BYTES];
    let mut packed_ptr = &mut packed[..packed_len];
    encode_binary_u64_be_fixed(&mut packed_ptr, value, packed_len)?;
    let mut tail = &mut buf[prefix_len..];
    let _ = unpack_padded_nibbles(&mut tail, &packed[..packed_len], tail_len, true, 0, &F::DIGITS)?;
    Ok(())
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn decode_nibble_int_fixed<F: NibbleFormat>(input: &mut &[u8], len: usize) -> Result<u64, Error> {
    const WORD_BYTES: usize = size_of::<u64>();
    const WORD_DIGITS: usize = WORD_BYTES * 2;

    let bytes = input.split_off(..len).ok_or_else(|| {
        cold_path();
        Error::UnexpectedEof
    })?;
    validate_nibble_int_fixed::<F>(bytes, len)?;
    let prefix_len = len.saturating_sub(WORD_DIGITS);
    if prefix_len != 0 && !all_bytes_eq(&bytes[..prefix_len], F::DIGITS[0]) {
        cold_path();
        return Err(Error::Invalid);
    }
    let tail = &bytes[prefix_len..];
    let packed_len = tail.len().div_ceil(2);
    let mut packed = [0u8; WORD_BYTES];
    let mut packed_ptr = &mut packed[..packed_len];
    let _ = pack_nibbles(&mut packed_ptr, tail, true, 0, &F::TABLE)?;
    let mut packed_input = &packed[..packed_len];
    decode_binary_u64_be_fixed(&mut packed_input, packed_len)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn encode_binary_u64_be_fixed(output: &mut &mut [u8], value: u64, len: usize) -> Result<(), Error> {
    encode_be_bytes(output, &value.to_be_bytes(), len, 0)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn decode_binary_u64_be_fixed(input: &mut &[u8], len: usize) -> Result<u64, Error> {
    Ok(u64::from_be_bytes(extend_be_bytes::<{ size_of::<u64>() }>(input, len, 0)?))
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_binary_i64_be_fixed(value: i64, len: usize) -> Result<(), Error> {
    if len == 0 || len > core::mem::size_of::<i64>() {
        cold_path();
        return Err(Error::Invalid);
    }

    let bits = len * 8;
    if bits < 64 {
        let min = -(1i128 << (bits - 1));
        let max = (1i128 << (bits - 1)) - 1;
        let value = i128::from(value);
        if value < min || value > max {
            cold_path();
            return Err(Error::Invalid);
        }
    }
    Ok(())
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn encode_binary_i64_be_fixed(output: &mut &mut [u8], value: i64, len: usize) -> Result<(), Error> {
    validate_binary_i64_be_fixed(value, len)?;
    encode_be_bytes(output, &value.to_be_bytes(), len, if value < 0 { 0xFF } else { 0x00 })
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn decode_binary_i64_be_fixed(input: &mut &[u8], len: usize) -> Result<i64, Error> {
    if len == 0 || len > core::mem::size_of::<i64>() {
        cold_path();
        return Err(Error::Invalid);
    }

    let fill = if input.first().is_some_and(|byte| byte & 0x80 != 0) {
        0xFF
    } else {
        0x00
    };
    Ok(i64::from_be_bytes(extend_be_bytes::<{ size_of::<i64>() }>(input, len, fill)?))
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn decode_signed_magnitude_i64(negative: bool, magnitude: u64) -> Result<i64, Error> {
    if !negative {
        return i64::try_from(magnitude).map_err(|_| {
            cold_path();
            Error::Invalid
        });
    }
    let limit = i64::MAX as u64 + 1;
    if magnitude > limit {
        cold_path();
        return Err(Error::Invalid);
    }
    if magnitude == limit {
        Ok(i64::MIN)
    } else {
        Ok(-(magnitude as i64))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        decode_binary_i64_be_fixed, decode_binary_u64_be_fixed, decode_nibble_int_fixed, decode_signed_magnitude_i64,
        encode_binary_i64_be_fixed, encode_binary_u64_be_fixed, encode_nibble_int_fixed, validate_nibble_int_fixed,
    };
    use crate::Error;
    use crate::primitive::nibble::{HexEbcdic, HexLower, HexUpper};

    fn encode<const N: usize>(f: impl FnOnce(&mut &mut [u8]) -> Result<(), Error>) -> Result<[u8; N], Error> {
        let mut out = [0u8; N];
        let mut out_ptr = out.as_mut_slice();
        f(&mut out_ptr)?;
        Ok(out)
    }

    fn decode_be<const N: usize>(input: &[u8]) -> Result<u64, Error> {
        let mut input = input;
        decode_binary_u64_be_fixed(&mut input, N)
    }

    fn decode_hex<F: crate::primitive::nibble::NibbleFormat, const N: usize>(input: &[u8]) -> Result<u64, Error> {
        let mut input = input;
        decode_nibble_int_fixed::<F>(&mut input, N)
    }

    fn decode_signed_be<const N: usize>(input: &[u8]) -> Result<i64, Error> {
        let mut input = input;
        decode_binary_i64_be_fixed(&mut input, N)
    }

    #[test]
    fn test_decode_signed_magnitude_i64() {
        assert_eq!(decode_signed_magnitude_i64(false, 0), Ok(0));
        assert_eq!(decode_signed_magnitude_i64(false, i64::MAX as u64), Ok(i64::MAX));
        assert_eq!(decode_signed_magnitude_i64(false, i64::MAX as u64 + 1), Err(Error::Invalid));
        assert_eq!(decode_signed_magnitude_i64(true, 1), Ok(-1));
        assert_eq!(decode_signed_magnitude_i64(true, i64::MAX as u64 + 1), Ok(i64::MIN));
        assert_eq!(decode_signed_magnitude_i64(true, i64::MAX as u64 + 2), Err(Error::Invalid));
    }

    #[test]
    fn test_validate_fixed_width_nibble_integer() {
        assert_eq!(validate_nibble_int_fixed::<HexUpper>(b"1F", 2), Ok(()));
        assert_eq!(validate_nibble_int_fixed::<HexUpper>(b"1g", 2), Err(Error::Invalid));
        assert_eq!(validate_nibble_int_fixed::<HexUpper>(b"1f", 2), Err(Error::Invalid));
        assert_eq!(validate_nibble_int_fixed::<HexLower>(b"AB", 2), Err(Error::Invalid));
        assert_eq!(validate_nibble_int_fixed::<HexEbcdic>(b"AF", 2), Err(Error::Invalid));
        assert_eq!(validate_nibble_int_fixed::<HexUpper>(b"1", 2), Err(Error::Invalid));
    }

    #[test]
    fn test_fixed_width_nibble_integer_codecs() {
        assert_eq!(encode::<2>(|out| encode_nibble_int_fixed::<HexUpper>(out, 0x00, 2)), Ok(*b"00"));
        assert_eq!(encode::<2>(|out| encode_nibble_int_fixed::<HexUpper>(out, 0xFF, 2)), Ok(*b"FF"));
        assert_eq!(encode::<2>(|out| encode_nibble_int_fixed::<HexLower>(out, 0xAB, 2)), Ok(*b"ab"));
        assert_eq!(
            encode::<2>(|out| encode_nibble_int_fixed::<HexEbcdic>(out, 0xAF, 2)),
            Ok(*b"\xC1\xC6")
        );
        assert_eq!(encode::<3>(|out| encode_nibble_int_fixed::<HexUpper>(out, 0xABC, 3)), Ok(*b"ABC"));
        assert_eq!(decode_hex::<HexUpper, 2>(b"00"), Ok(0x00));
        assert_eq!(decode_hex::<HexUpper, 2>(b"FF"), Ok(0xFF));
        assert_eq!(decode_hex::<HexLower, 2>(b"ab"), Ok(0xAB));
        assert_eq!(decode_hex::<HexEbcdic, 2>(b"\xC1\xC6"), Ok(0xAF));
        assert_eq!(decode_hex::<HexUpper, 3>(b"ABC"), Ok(0xABC));

        const WIDE: usize = size_of::<u64>() * 2 + 1;
        let zeros = [b'0'; WIDE];
        let mut wide_zero = vec![b'0'; WIDE];
        wide_zero[0] = b'1';
        assert_eq!(decode_hex::<HexUpper, WIDE>(&zeros), Ok(0));
        assert_eq!(decode_hex::<HexUpper, WIDE>(&wide_zero), Err(Error::Invalid));

        assert_eq!(
            encode::<2>(|out| encode_nibble_int_fixed::<HexUpper>(out, 0x100, 2)),
            Err(Error::Invalid)
        );
        assert_eq!(
            encode::<1>(|out| encode_nibble_int_fixed::<HexUpper>(out, 0x01, 2)),
            Err(Error::BufferOverflow)
        );
        assert_eq!(decode_hex::<HexUpper, 2>(b"1g"), Err(Error::Invalid));
        assert_eq!(decode_hex::<HexUpper, 2>(b"1f"), Err(Error::Invalid));
        assert_eq!(decode_hex::<HexLower, 2>(b"AB"), Err(Error::Invalid));
        assert_eq!(decode_hex::<HexEbcdic, 2>(b"AF"), Err(Error::Invalid));
        assert_eq!(decode_hex::<HexUpper, 2>(b"F"), Err(Error::UnexpectedEof));
    }

    #[test]
    fn test_fixed_width_binary_integer_codecs() {
        assert_eq!(encode::<1>(|out| encode_binary_u64_be_fixed(out, 0x00, 1)), Ok(*b"\x00"));
        assert_eq!(encode::<1>(|out| encode_binary_u64_be_fixed(out, 0xFF, 1)), Ok(*b"\xFF"));
        assert_eq!(encode::<2>(|out| encode_binary_u64_be_fixed(out, 0x1234, 2)), Ok(*b"\x12\x34"));
        assert_eq!(encode::<2>(|out| encode_binary_u64_be_fixed(out, 0xFFFF, 2)), Ok(*b"\xFF\xFF"));
        assert_eq!(
            encode::<9>(|out| encode_binary_u64_be_fixed(out, 0x1234, 9)),
            Ok(*b"\x00\x00\x00\x00\x00\x00\x00\x12\x34")
        );
        assert_eq!(decode_be::<1>(b"\x00"), Ok(0x00));
        assert_eq!(decode_be::<1>(b"\xAB"), Ok(0xAB));
        assert_eq!(decode_be::<2>(b"\x12\x34"), Ok(0x1234));
        assert_eq!(decode_be::<2>(b"\xFF\xFF"), Ok(0xFFFF));
        assert_eq!(decode_be::<9>(b"\x00\x00\x00\x00\x00\x00\x00\x12\x34"), Ok(0x1234));

        assert_eq!(
            encode::<1>(|out| encode_binary_u64_be_fixed(out, 0x12, 2)),
            Err(Error::BufferOverflow)
        );
        assert_eq!(decode_be::<2>(b"\x12"), Err(Error::UnexpectedEof));
        assert_eq!(decode_be::<9>(b"\x01\x00\x00\x00\x00\x00\x00\x00\x00"), Err(Error::Invalid));
    }

    #[test]
    fn test_fixed_width_signed_binary_integer_codecs() {
        for &(value, expected) in &[(0_i64, *b"\x00"), (127, *b"\x7F"), (-128, *b"\x80"), (-1, *b"\xFF")] {
            assert_eq!(encode::<1>(|out| encode_binary_i64_be_fixed(out, value, 1)), Ok(expected));
            assert_eq!(decode_signed_be::<1>(&expected), Ok(value));
        }

        for &(value, expected) in &[
            (0_i64, *b"\x00\x00"),
            (0x1234, *b"\x12\x34"),
            (-2, *b"\xFF\xFE"),
            (i16::MIN as i64, *b"\x80\x00"),
            (i16::MAX as i64, *b"\x7F\xFF"),
        ] {
            assert_eq!(encode::<2>(|out| encode_binary_i64_be_fixed(out, value, 2)), Ok(expected));
            assert_eq!(decode_signed_be::<2>(&expected), Ok(value));
        }

        assert_eq!(encode::<1>(|out| encode_binary_i64_be_fixed(out, 128, 1)), Err(Error::Invalid));
        assert_eq!(encode::<1>(|out| encode_binary_i64_be_fixed(out, -129, 1)), Err(Error::Invalid));
        assert_eq!(encode::<1>(|out| encode_binary_i64_be_fixed(out, 1, 2)), Err(Error::BufferOverflow));
        assert_eq!(decode_signed_be::<2>(b"\x12"), Err(Error::UnexpectedEof));
    }
}

#[cfg(test)]
mod proptests {
    use proptest::test_runner::TestCaseResult;
    use proptest::{prop_assert_eq, proptest};

    use super::{decode_binary_i64_be_fixed, encode_binary_i64_be_fixed};

    fn roundtrip_signed_be<const N: usize>(value: i64) -> TestCaseResult {
        let mut output = [0u8; N];
        let mut out_ptr = output.as_mut_slice();
        encode_binary_i64_be_fixed(&mut out_ptr, value, N).unwrap();
        let mut input = &output[..];
        prop_assert_eq!(decode_binary_i64_be_fixed(&mut input, N), Ok(value));
        Ok(())
    }

    proptest! {
        #[test]
        fn signed_binary_be_roundtrips_i8(value in i8::MIN..=i8::MAX) {
            roundtrip_signed_be::<1>(i64::from(value))?;
        }

        #[test]
        fn signed_binary_be_roundtrips_i16(value in i16::MIN..=i16::MAX) {
            roundtrip_signed_be::<2>(i64::from(value))?;
        }

        #[test]
        fn signed_binary_be_roundtrips_i32(value: i32) {
            roundtrip_signed_be::<4>(i64::from(value))?;
        }

        #[test]
        fn signed_binary_be_roundtrips_i64(value: i64) {
            roundtrip_signed_be::<8>(value)?;
        }
    }
}
