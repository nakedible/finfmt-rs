use core::marker::PhantomData;

use crate::primitive::text::{truncate_bytes, truncate_str};
use crate::{Error, ScalarFmt};

/// Explicitly cut a value to at most `MAX_LEN` before inner validation, length
/// prediction and encoding. Keeps the left end, or the right end when
/// `KEEP_RIGHT` is true; padding and framing remain the inner format's job.
///
/// `MAX_LEN` is in the value's own units, not what a length prefix counts.
/// String values are cut by characters
/// and never split a character; wire text is single-byte, so a character is one
/// wire byte. Byte values are cut by bytes. Discarded input is not validated.
///
/// Typed numeric methods delegate directly to `F`, without truncation. All
/// decoding methods also delegate, including their borrowing and scratch behavior.
///
/// This format writes `ABCD` for `ABCDE`, and rejects inputs shorter than four
/// characters:
///
/// ```
/// use finfmt::{Ascii, Field, Fixed, Truncate};
/// type Name = Truncate<Field<Ascii<4, 4>, Fixed<4>>, 4>;
/// ```
pub struct Truncate<F, const MAX_LEN: usize, const KEEP_RIGHT: bool = false>(PhantomData<F>);

impl<F: ScalarFmt, const MAX_LEN: usize, const KEEP_RIGHT: bool> ScalarFmt for Truncate<F, MAX_LEN, KEEP_RIGHT> {
    const TAKES_REST: bool = F::TAKES_REST;

    #[inline(always)]
    fn encoded_len(input: &[u8]) -> Result<usize, Error> {
        F::encoded_len(truncate_bytes(input, MAX_LEN, KEEP_RIGHT))
    }

    #[inline(always)]
    fn encode(output: &mut &mut [u8], scratch: &mut [u8], input: &[u8]) -> Result<(), Error> {
        F::encode(output, scratch, truncate_bytes(input, MAX_LEN, KEEP_RIGHT))
    }

    #[inline(always)]
    fn decode<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<&'a [u8], Error> {
        F::decode(input, scratch)
    }

    #[inline(always)]
    fn encoded_len_str(input: &str) -> Result<usize, Error> {
        F::encoded_len_str(truncate_str(input, MAX_LEN, KEEP_RIGHT))
    }

    #[inline(always)]
    fn encode_str(output: &mut &mut [u8], scratch: &mut [u8], input: &str) -> Result<(), Error> {
        F::encode_str(output, scratch, truncate_str(input, MAX_LEN, KEEP_RIGHT))
    }

    #[inline(always)]
    fn decode_str<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<&'a str, Error> {
        F::decode_str(input, scratch)
    }

    #[inline(always)]
    fn encoded_len_u64(input: u64) -> Result<usize, Error> {
        F::encoded_len_u64(input)
    }

    #[inline(always)]
    fn encoded_len_usize(input: usize) -> Result<usize, Error> {
        F::encoded_len_usize(input)
    }

    #[inline(always)]
    fn encoded_len_i64(input: i64) -> Result<usize, Error> {
        F::encoded_len_i64(input)
    }

    #[inline(always)]
    fn encode_u64(output: &mut &mut [u8], scratch: &mut [u8], input: u64) -> Result<(), Error> {
        F::encode_u64(output, scratch, input)
    }

    #[inline(always)]
    fn encode_usize(output: &mut &mut [u8], scratch: &mut [u8], input: usize) -> Result<(), Error> {
        F::encode_usize(output, scratch, input)
    }

    #[inline(always)]
    fn encode_i64(output: &mut &mut [u8], scratch: &mut [u8], input: i64) -> Result<(), Error> {
        F::encode_i64(output, scratch, input)
    }

    #[inline(always)]
    fn decode_u64<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<u64, Error> {
        F::decode_u64(input, scratch)
    }

    #[inline(always)]
    fn decode_usize<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<usize, Error> {
        F::decode_usize(input, scratch)
    }

    #[inline(always)]
    fn decode_i64<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<i64, Error> {
        F::decode_i64(input, scratch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Ascii, AsciiLength, Binary, Ebcdic037, Field, Fixed, FixedBinaryBe, FixedSignedBinaryBe};

    type Text = Field<Ascii<0, 8>, AsciiLength<1>>;

    fn encode<F: ScalarFmt>(input: &[u8]) -> Result<Vec<u8>, Error> {
        let mut output = [0xAA; 32];
        let mut out = output.as_mut_slice();
        F::encode(&mut out, &mut [][..], input)?;
        let written = 32 - out.len();
        assert_eq!(F::encoded_len(input), Ok(written));
        assert!(out.iter().all(|&byte| byte == 0xAA));
        Ok(output[..written].to_vec())
    }

    #[test]
    fn truncation_precedes_validation_and_framing() {
        assert_eq!(encode::<Truncate<Text, 4>>(b"ABCDE\xFF"), Ok(b"4ABCD".to_vec()));
        assert_eq!(encode::<Truncate<Text, 4, true>>(b"\xFFABCDE"), Ok(b"4BCDE".to_vec()));
        assert_eq!(encode::<Truncate<Text, 4>>(b"\xFFABCDE"), Err(Error::Invalid));
        assert_eq!(encode::<Truncate<Text, 4>>(b"AB"), Ok(b"2AB".to_vec()));
        assert_eq!(encode::<Truncate<Text, 0>>(b"\xFF"), Ok(b"0".to_vec()));
        assert_eq!(encode::<Truncate<Text, { usize::MAX }>>(b"ABC"), Ok(b"3ABC".to_vec()));
        type Exact = Truncate<Field<Ascii<4, 4>, Fixed<4>>, 4>;
        assert_eq!(Exact::encoded_len(b"AB"), Err(Error::InvalidValueLength));
        assert_eq!(encode::<Exact>(b"AB"), Err(Error::InvalidValueLength));
        assert_eq!(encode::<Exact>(b"ABCDE"), Ok(b"ABCD".to_vec()));
        assert_eq!(
            Exact::encode(&mut &mut [0; 3][..], &mut [][..], b"ABCDE"),
            Err(Error::BufferOverflow)
        );
    }

    #[test]
    fn decoding_keeps_inner_length_and_borrows() {
        let wire = b"5ABCDE!";
        let mut input = wire.as_slice();
        let mut scratch = [0xAA; 12];
        let mut space = scratch.as_mut_slice();
        let value = Truncate::<Text, 4>::decode(&mut input, &mut space).unwrap();
        assert_eq!(value, b"ABCDE");
        assert_eq!(value.as_ptr(), wire[1..].as_ptr());
        assert_eq!(input, b"!");
        assert_eq!(space.len(), 12);

        type Converted = Truncate<Field<Ascii<0, 8>, AsciiLength<1>, Ebcdic037>, 4>;
        let scratch_start = space.as_ptr();
        let mut input = &b"5\xC1\xC2\xC3\xC4\xC5!"[..];
        let value = Converted::decode(&mut input, &mut space).unwrap();
        assert_eq!(value, b"ABCDE");
        assert_eq!(value.as_ptr(), scratch_start);
        assert_eq!(input, b"!");
        assert_eq!(space, &[0xAA; 7]);
    }

    struct StringOnly;
    impl ScalarFmt for StringOnly {
        fn encoded_len(_: &[u8]) -> Result<usize, Error> {
            Err(Error::Internal)
        }
        fn encode(_: &mut &mut [u8], _: &mut [u8], _: &[u8]) -> Result<(), Error> {
            Err(Error::Internal)
        }
        fn decode<'a>(_: &mut &'a [u8], _: &mut &'a mut [u8]) -> Result<&'a [u8], Error> {
            Err(Error::Internal)
        }
        fn encoded_len_str(input: &str) -> Result<usize, Error> {
            Ok(input.len())
        }
        fn encode_str(output: &mut &mut [u8], _: &mut [u8], input: &str) -> Result<(), Error> {
            crate::primitive::bytes::copy_bytes(output, input.as_bytes()).map(|_| ())
        }
        fn decode_str<'a>(input: &mut &'a [u8], _: &mut &'a mut [u8]) -> Result<&'a str, Error> {
            core::str::from_utf8(core::mem::take(input)).map_err(|_| Error::Invalid)
        }
    }

    #[test]
    fn strings_preserve_specialization_and_count_characters() {
        type Left = Truncate<StringOnly, 2>;
        type Right = Truncate<StringOnly, 2, true>;
        assert_eq!(Left::encoded_len_str("éXY"), Ok(3));
        assert_eq!(Right::encoded_len_str("XYé"), Ok(3));
        assert_eq!(Left::encoded_len_str("Xé€"), Ok(3));
        let mut output = [0xAA; 8];
        let mut out = output.as_mut_slice();
        Left::encode_str(&mut out, &mut [][..], "éXY").unwrap();
        Right::encode_str(&mut out, &mut [][..], "XYé€").unwrap();
        assert!(out.is_empty());
        assert_eq!(output, *"éXé€".as_bytes());
        let mut input = "ABCDE".as_bytes();
        assert_eq!(Left::decode_str(&mut input, &mut &mut [][..]), Ok("ABCDE"));
        assert!(input.is_empty());
    }

    #[test]
    fn typed_numbers_delegate_without_truncation_or_scratch() {
        type Unsigned = Truncate<FixedBinaryBe<1>, 0>;
        type Signed = Truncate<FixedSignedBinaryBe<1>, 0>;
        assert_eq!(Unsigned::encoded_len_u64(255), Ok(1));
        assert_eq!(Unsigned::encoded_len_usize(255), Ok(1));
        assert_eq!(Signed::encoded_len_i64(-128), Ok(1));
        let mut output = [0u8; 3];
        let mut out = output.as_mut_slice();
        Unsigned::encode_u64(&mut out, &mut [][..], 255).unwrap();
        Unsigned::encode_usize(&mut out, &mut [][..], 254).unwrap();
        Signed::encode_i64(&mut out, &mut [][..], -128).unwrap();
        assert!(out.is_empty());
        assert_eq!(output, [255, 254, 128]);
        let mut input = output.as_slice();
        assert_eq!(Unsigned::decode_u64(&mut input, &mut &mut [][..]), Ok(255));
        assert_eq!(Unsigned::decode_usize(&mut input, &mut &mut [][..]), Ok(254));
        assert_eq!(Signed::decode_i64(&mut input, &mut &mut [][..]), Ok(-128));
        assert!(input.is_empty());
    }

    proptest::proptest! {
        #[test]
        fn arbitrary_bytes_keep_size_and_wire_consistent(input in proptest::collection::vec(proptest::prelude::any::<u8>(), 0..64)) {
            type Inner = Field<Binary<0, 8>, AsciiLength<1>>;
            let count = input.len().min(8);
            for (actual, expected) in [
                (encode::<Truncate<Inner, 8>>(&input), &input[..count]),
                (encode::<Truncate<Inner, 8, true>>(&input), &input[input.len() - count..]),
            ] {
                let wire = actual.unwrap();
                proptest::prop_assert_eq!(wire[0], b'0' + count as u8);
                let mut cursor = wire.as_slice();
                proptest::prop_assert_eq!(Inner::decode(&mut cursor, &mut &mut [][..]).unwrap(), expected);
                proptest::prop_assert!(cursor.is_empty());
            }
        }
    }
}
