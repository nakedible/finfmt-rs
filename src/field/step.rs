use core::marker::PhantomData;

use super::Check;
use crate::Error;
use crate::utils::{cold_path, split_scratch};

/// A transform between validated semantic data and its byte representation.
/// Encoding assumes the caller established the transform's input repertoire;
/// decoding checks untrusted representation details before depending on them.
/// Chained steps must agree on the units at each intermediate boundary.
pub trait Step {
    /// Encoding can transform the bytes in place without changing their length.
    const ENCODE_IN_PLACE: bool = false;

    /// Exact encoded byte count from logical input length. Built-in steps use
    /// bytes, except `Ebcdic1142`, which uses Unicode scalar values.
    fn encoded_len(input_len: usize) -> Result<usize, Error>;
    /// Upper bound on the final decoded byte length for this many encoded bytes.
    /// This excludes scratch consumed by intermediate steps. Decoding does not
    /// call this sizing helper; each step reserves its own output when needed.
    /// Chains pass upper bounds, so odd lengths and similar data-shape conditions
    /// must be checked by `decode`, not rejected during capacity calculation.
    fn decoded_max_len(input_len: usize) -> Result<usize, Error>;

    fn encode<'a>(output: &mut &'a mut [u8], scratch: &mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error>;

    /// Decode an already framed representation. `semantic_len`, when known, is
    /// the required decoded logical length, not a byte-capacity limit. Padding
    /// steps preserve at least that much and never discard non-padding data;
    /// the field composer checks the resulting semantic length.
    /// Borrow input when possible; otherwise reserve output from `scratch`,
    /// advancing it so later steps can allocate disjoint regions.
    fn decode<'a>(input: &'a [u8], scratch: &mut &'a mut [u8], semantic_len: Option<usize>) -> Result<&'a [u8], Error>;

    #[inline(always)]
    fn encode_in_place(_buf: &mut [u8]) -> Result<(), Error> {
        cold_path();
        Err(Error::Internal)
    }
}

/// Encode through `A` then `B`, and decode in reverse order.
/// Intermediate boundaries use compatible byte units. `A::encoded_len` applied
/// to the input byte count must bound its encoded size; this may reserve extra
/// scratch for UTF-8 input to `Ebcdic1142` without rescanning its characters.
pub struct Chain<A, B>(PhantomData<(A, B)>);
/// Validate encoded bytes before decoding with `S`; encoding passes through.
/// Semantic-length errors from the reused check become invalid wire data.
pub struct DecodeCheck<S, C>(PhantomData<(S, C)>);

impl<First: Step, Rest: Step> Step for Chain<First, Rest> {
    const ENCODE_IN_PLACE: bool = First::ENCODE_IN_PLACE && Rest::ENCODE_IN_PLACE;

    #[inline(always)]
    fn encoded_len(input_len: usize) -> Result<usize, Error> {
        Rest::encoded_len(First::encoded_len(input_len)?)
    }

    #[inline(always)]
    fn decoded_max_len(input_len: usize) -> Result<usize, Error> {
        First::decoded_max_len(Rest::decoded_max_len(input_len)?)
    }

    #[inline(always)]
    fn encode<'a>(output: &mut &'a mut [u8], scratch: &mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
        if Rest::ENCODE_IN_PLACE {
            let buf = First::encode(output, scratch, input)?;
            Rest::encode_in_place(buf)?;
            return Ok(buf);
        }
        let mid_len = First::encoded_len(input.len())?;
        let (mid_buf, scratch) = split_scratch(scratch, mid_len)?;
        let mid = First::encode(&mut &mut *mid_buf, scratch, input)?;
        Rest::encode(output, scratch, mid)
    }

    #[inline(always)]
    fn decode<'a>(input: &'a [u8], scratch: &mut &'a mut [u8], semantic_len: Option<usize>) -> Result<&'a [u8], Error> {
        let rest_semantic_len = match semantic_len {
            Some(semantic_len) => Some(First::encoded_len(semantic_len)?),
            None => None,
        };
        let mid = Rest::decode(input, scratch, rest_semantic_len)?;
        First::decode(mid, scratch, semantic_len)
    }

    #[inline(always)]
    fn encode_in_place(buf: &mut [u8]) -> Result<(), Error> {
        First::encode_in_place(buf)?;
        Rest::encode_in_place(buf)
    }
}

impl<S: Step, C: Check> Step for DecodeCheck<S, C> {
    const ENCODE_IN_PLACE: bool = S::ENCODE_IN_PLACE;

    #[inline(always)]
    fn encoded_len(input_len: usize) -> Result<usize, Error> {
        S::encoded_len(input_len)
    }

    #[inline(always)]
    fn decoded_max_len(input_len: usize) -> Result<usize, Error> {
        S::decoded_max_len(input_len)
    }

    #[inline(always)]
    fn encode<'a>(output: &mut &'a mut [u8], scratch: &mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
        S::encode(output, scratch, input)
    }

    #[inline(always)]
    fn decode<'a>(input: &'a [u8], scratch: &mut &'a mut [u8], semantic_len: Option<usize>) -> Result<&'a [u8], Error> {
        C::validate(input).map_err(|error| {
            cold_path();
            if error == Error::InvalidValueLength {
                Error::Invalid
            } else {
                error
            }
        })?;
        S::decode(input, scratch, semantic_len)
    }

    #[inline(always)]
    fn encode_in_place(buf: &mut [u8]) -> Result<(), Error> {
        S::encode_in_place(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::Chain;
    use crate::primitive::nibble::HexUpper as HexDigits;
    use crate::*;

    fn encode_without_scratch<F: ScalarFmt>(input: &[u8], expected: &[u8]) {
        let mut output = [0xEE; 16];
        let mut out = &mut output[..];
        F::encode(&mut out, &mut [][..], input).unwrap();
        assert_eq!(out.len(), 16 - expected.len());
        assert_eq!(&output[..expected.len()], expected);
    }

    #[test]
    fn in_place_encoding_composes_without_scratch() {
        encode_without_scratch::<Field<Ascii<3, 3>, Fixed<3>>>(b"ABC", b"ABC");
        encode_without_scratch::<Field<Ascii<3, 3>, Fixed<3>, Ebcdic037>>(b"ABC", &[0xC1, 0xC2, 0xC3]);
        encode_without_scratch::<Field<Ascii<3, 3>, Fixed<3>, Chain<Identity, Ebcdic037>>>(b"ABC", &[0xC1, 0xC2, 0xC3]);
        encode_without_scratch::<Field<Ascii<0, 8>, WireFixed<8>, Chain<PadRight<8>, Ebcdic037>>>(
            b"ABC",
            &[0xC1, 0xC2, 0xC3, 0x40, 0x40, 0x40, 0x40, 0x40],
        );
        encode_without_scratch::<PaddedField<Ascii<3, 3>, Fixed<3>, Ebcdic037, 5, b' '>>(b"ABC", &[0xC1, 0xC2, 0xC3, b' ', b' ']);
    }

    #[test]
    fn even_size_overflow_is_reported() {
        for len in [0, 1, 2, 3, usize::MAX - 1] {
            assert_eq!(PadRightEven::<b' '>::encoded_len(len), Ok(len + len % 2));
            assert_eq!(PadLeftEven::<b'0'>::encoded_len(len), Ok(len + len % 2));
        }
        assert_eq!(PadRightEven::<b' '>::encoded_len(usize::MAX), Err(Error::BufferOverflow));
        assert_eq!(PadLeftEven::<b'0'>::encoded_len(usize::MAX), Err(Error::BufferOverflow));
    }

    #[test]
    fn oversized_unpack_request_is_invalid() {
        assert_eq!(
            UnpackNibbles::<HexDigits>::decode(&[], &mut &mut [][..], Some(usize::MAX / 2 + 1)),
            Err(Error::Invalid)
        );
    }

    #[test]
    fn wire_check_length_is_invalid_wire_data() {
        assert_eq!(
            DecodeCheck::<Identity, Binary<2, 2>>::decode(b"A", &mut &mut [][..], None),
            Err(Error::Invalid)
        );
    }

    struct InternalCheck;
    impl Check for InternalCheck {
        fn validate(_: &[u8]) -> Result<usize, Error> {
            Err(Error::Internal)
        }
    }

    #[test]
    fn decode_check_preserves_internal_errors_and_encode_passes_through() {
        assert_eq!(
            DecodeCheck::<Identity, InternalCheck>::decode(b"A", &mut &mut [][..], None),
            Err(Error::Internal)
        );
        let mut output = [0; 1];
        assert_eq!(
            DecodeCheck::<Identity, Numeric<1, 1>>::encode(&mut &mut output[..], &mut [][..], b"?").as_deref(),
            Ok(&b"?"[..])
        );
    }

    #[test]
    fn decoded_capacity_is_an_upper_bound_not_a_shape_check() {
        type F = Field<Binary<1, 1>, WireFixed<3>, chain!(UnpackNibbles<HexDigits>, PadRight<3, b' '>)>;
        let mut scratch = [0; 8];
        assert_eq!(F::decode(&mut &b"AB "[..], &mut &mut scratch[..]), Ok(&[0xAB][..]));
    }
}

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    use crate::primitive::nibble::HexUpper;
    use crate::{Binary, Field, PadRight, Rest, ScalarFmt, UnpackNibbles};

    proptest! {
        #[test]
        fn padded_hex_roundtrip(value in prop::collection::vec(any::<u8>(), 0..=32)) {
            type F = Field<Binary<0,32>, Rest, crate::chain!(UnpackNibbles<HexUpper>, PadRight<3>)>;
            let mut output = [0;64];
            let mut scratch = [0;128];
            let mut out = output.as_mut_slice();
            F::encode(&mut out, scratch.as_mut_slice(), &value).unwrap();
            let used = 64 - out.len();
            prop_assert_eq!(F::encoded_len(&value), Ok(used));
            let mut wire = &output[..used];
            prop_assert_eq!(F::decode(&mut wire, &mut scratch.as_mut_slice()).unwrap(), value.as_slice());
            prop_assert!(wire.is_empty());
        }
    }
}
