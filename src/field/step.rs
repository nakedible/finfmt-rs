use core::marker::PhantomData;

use super::Check;
use crate::Error;
use crate::utils::{cold_path, take_scratch};

/// A transform between validated semantic data and its byte representation.
/// Encoding assumes the caller established the transform's input repertoire;
/// decoding checks untrusted representation details before depending on them.
/// Chained steps must agree on the units at each intermediate boundary.
pub trait Step {
    /// Encoding can transform the bytes in place without changing their length.
    const INPLACE: bool = false;

    /// Exact encoded byte count from logical input length. Built-in steps use
    /// bytes, except `Ebcdic1142`, which uses Unicode scalar values.
    fn encoded_len(input_len: usize) -> Result<usize, Error>;
    /// Byte capacity sufficient to decode up to this many encoded bytes.
    /// Chains pass upper bounds, so odd lengths and similar data-shape conditions
    /// must be checked by `decode`, not rejected during capacity calculation.
    fn decoded_max_len(input_len: usize) -> Result<usize, Error>;

    fn encode<'a>(output: &mut &'a mut [u8], scratch: &mut &mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error>;

    /// Decode an already framed representation. `output_len`, when known, is
    /// the original logical input length, not a byte-capacity limit. Padding
    /// steps preserve at least that much and never discard non-padding data;
    /// the field composer checks the resulting semantic length.
    fn decode<'a>(
        input: &'a [u8],
        output: &mut &'a mut [u8],
        scratch: &mut &'a mut [u8],
        output_len: Option<usize>,
    ) -> Result<&'a [u8], Error>;

    #[inline(always)]
    fn encode_inplace(_buf: &mut [u8]) -> Result<(), Error> {
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
    const INPLACE: bool = First::INPLACE && Rest::INPLACE;

    #[inline(always)]
    fn encoded_len(input_len: usize) -> Result<usize, Error> {
        Rest::encoded_len(First::encoded_len(input_len)?)
    }

    #[inline(always)]
    fn decoded_max_len(input_len: usize) -> Result<usize, Error> {
        First::decoded_max_len(Rest::decoded_max_len(input_len)?)
    }

    #[inline(always)]
    fn encode<'a>(output: &mut &'a mut [u8], scratch: &mut &mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
        let mid_len = First::encoded_len(input.len())?;
        let mid_buf = take_scratch(scratch, mid_len)?;
        let mut mid_out = mid_buf;
        let mid = First::encode(&mut mid_out, scratch, input)?;
        Rest::encode(output, scratch, mid)
    }

    #[inline(always)]
    fn decode<'a>(
        input: &'a [u8],
        output: &mut &'a mut [u8],
        scratch: &mut &'a mut [u8],
        output_len: Option<usize>,
    ) -> Result<&'a [u8], Error> {
        let rest_output_len = match output_len {
            Some(output_len) => Some(First::encoded_len(output_len)?),
            None => None,
        };
        let mid_cap = match rest_output_len {
            Some(len) => len,
            None => Rest::decoded_max_len(input.len())?,
        };
        let mid_buf = take_scratch(scratch, mid_cap)?;
        let mut mid_out = mid_buf;
        let mid = Rest::decode(input, &mut mid_out, scratch, rest_output_len)?;
        First::decode(mid, output, scratch, output_len)
    }

    #[inline(always)]
    fn encode_inplace(buf: &mut [u8]) -> Result<(), Error> {
        First::encode_inplace(buf)?;
        Rest::encode_inplace(buf)
    }
}

impl<S: Step, C: Check> Step for DecodeCheck<S, C> {
    const INPLACE: bool = S::INPLACE;

    #[inline(always)]
    fn encoded_len(input_len: usize) -> Result<usize, Error> {
        S::encoded_len(input_len)
    }

    #[inline(always)]
    fn decoded_max_len(input_len: usize) -> Result<usize, Error> {
        S::decoded_max_len(input_len)
    }

    #[inline(always)]
    fn encode<'a>(output: &mut &'a mut [u8], scratch: &mut &mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
        S::encode(output, scratch, input)
    }

    #[inline(always)]
    fn decode<'a>(
        input: &'a [u8],
        output: &mut &'a mut [u8],
        scratch: &mut &'a mut [u8],
        output_len: Option<usize>,
    ) -> Result<&'a [u8], Error> {
        C::validate(input).map_err(|error| {
            cold_path();
            if error == Error::InvalidValueLength {
                Error::Invalid
            } else {
                error
            }
        })?;
        S::decode(input, output, scratch, output_len)
    }

    #[inline(always)]
    fn encode_inplace(buf: &mut [u8]) -> Result<(), Error> {
        S::encode_inplace(buf)
    }
}

#[cfg(test)]
mod tests {
    use crate::primitive::nibble::HexUpper as HexDigits;
    use crate::*;

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
            UnpackNibbles::<HexDigits>::decode(&[], &mut &mut [][..], &mut &mut [][..], Some(usize::MAX / 2 + 1)),
            Err(Error::Invalid)
        );
    }

    #[test]
    fn wire_check_length_is_invalid_wire_data() {
        assert_eq!(
            DecodeCheck::<Identity, Binary<2, 2>>::decode(b"A", &mut &mut [][..], &mut &mut [][..], None),
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
            DecodeCheck::<Identity, InternalCheck>::decode(b"A", &mut &mut [][..], &mut &mut [][..], None),
            Err(Error::Internal)
        );
        let mut output = [0; 1];
        assert_eq!(
            DecodeCheck::<Identity, Numeric<1, 1>>::encode(&mut &mut output[..], &mut &mut [][..], b"?").as_deref(),
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
            F::encode(&mut out, &mut scratch.as_mut_slice(), &value).unwrap();
            let used = 64 - out.len();
            prop_assert_eq!(F::encoded_len(&value), Ok(used));
            let mut wire = &output[..used];
            prop_assert_eq!(F::decode(&mut wire, &mut scratch.as_mut_slice()).unwrap(), value.as_slice());
            prop_assert!(wire.is_empty());
        }
    }
}
