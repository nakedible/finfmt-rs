use core::marker::PhantomData;

use super::Check;
use crate::Error;
use crate::utils::{cold_path, length_as_invalid, split_scratch};

/// A transform between validated semantic data and its byte representation.
/// Encoding assumes the caller established the transform's input repertoire;
/// decoding checks untrusted representation details before depending on them.
/// Chained steps must agree on the units at each intermediate boundary.
///
/// Every step's output length must be an arithmetic function of its input
/// length (`encoded_len`), so framing can compute lengths without transforming.
pub trait Step {
    /// Encoding can transform the bytes in place without changing their length.
    ///
    /// This is a speed optimisation: a `Chain` ending in such a step lets the
    /// earlier steps write straight into output and transforms them there,
    /// skipping a scratch stage. For padded CP037 text this is 12–67% faster on
    /// 8–32 byte fields; the gain fades by 64 bytes. It also lets such chains
    /// encode without scratch, though callers always provide scratch.
    const ENCODE_IN_PLACE: bool = false;

    /// Encoding can finish in place by transforming the bytes where they are
    /// and writing more after them, as `PadRight` does. A `Chain` ending in
    /// such a step lets the earlier steps write straight into output too, but
    /// it costs a little more than [`Step::ENCODE_IN_PLACE`], which it
    /// includes.
    const ENCODE_APPENDING: bool = Self::ENCODE_IN_PLACE;

    /// Encoding copies the input unchanged, so a `Chain` starting with this
    /// step skips staging its output.
    const ENCODE_UNCHANGED: bool = false;

    /// The step contains the [`Count`] marker. Only `Count` and the steps that
    /// wrap others set it.
    const HAS_COUNT: bool = false;

    /// Exact encoded byte count from logical input length. Built-in steps use
    /// bytes, except `Ebcdic1142`, which uses Unicode scalar values.
    fn encoded_len(input_len: usize) -> Result<usize, Error>;

    /// The length a field's length counts, for this input length: the length
    /// at the [`Count`] marker, or the output length without one.
    #[inline(always)]
    fn counted_len(input_len: usize) -> Result<usize, Error> {
        Self::encoded_len(input_len)
    }

    /// The output length for a counted length `count`: the steps after the
    /// [`Count`] marker applied to it. Without a marker the count is the
    /// output length.
    #[inline(always)]
    fn counted_wire_len(count: usize) -> Result<usize, Error> {
        Ok(count)
    }

    fn encode<'a>(output: &mut &'a mut [u8], scratch: &mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error>;

    /// Decode an already framed representation. `len`, when known, is the
    /// exact length this step decodes to, which the framing fixed: the step is
    /// after the [`Count`] marker. Steps whose padding the content cannot
    /// identify use it to split value from padding exactly; without it they
    /// strip padding by content.
    /// Borrow input when possible; otherwise reserve output from `scratch`,
    /// advancing it so later steps can allocate disjoint regions.
    fn decode<'a>(input: &'a [u8], scratch: &mut &'a mut [u8], len: Option<usize>) -> Result<&'a [u8], Error>;

    /// Decode a whole field's representation given its counted length, when
    /// the framing states one: steps after the [`Count`] marker get their exact
    /// lengths, steps before it get none. Without a marker no step gets one.
    #[inline(always)]
    fn decode_counted<'a>(input: &'a [u8], scratch: &mut &'a mut [u8], _count: Option<usize>) -> Result<&'a [u8], Error> {
        Self::decode(input, scratch, None)
    }

    /// Finish encoding in place: `buf` is the step's whole output,
    /// `encoded_len(input_len)` bytes, starting with the `input_len` input
    /// bytes. Only for steps with [`Step::ENCODE_APPENDING`].
    #[inline(always)]
    fn encode_in_place(_buf: &mut [u8], _input_len: usize) -> Result<(), Error> {
        cold_path();
        Err(Error::Internal)
    }
}

/// Encode through `A` then `B`, and decode in reverse order.
/// Intermediate boundaries use compatible byte units. `A::encoded_len` applied
/// to the input byte count must bound its encoded size; this may reserve extra
/// scratch for UTF-8 input to `Ebcdic1142` without rescanning its characters.
/// At most one of the two may contain the [`Count`] marker.
pub struct Chain<A, B>(PhantomData<(A, B)>);
/// Validate encoded bytes before decoding with `S`; encoding passes through.
/// Semantic-length errors from the reused check become invalid wire data.
pub struct DecodeCheck<S, C>(PhantomData<(S, C)>);

/// Marks where in a step chain a field's length is counted. Steps before it
/// are counted, steps after it are not: `chain!(PadRight<8>, Count,
/// PackNibbles<…>)` counts padded digits, and `chain!(Count, PadRight<4>)`
/// counts the value without its padding. A chain without the marker counts its
/// output, the wire bytes. A chain has at most one marker.
///
/// Decoding gives the steps after the marker their exact lengths, derived from
/// the count, so padding after the marker is split off exactly.
pub struct Count;

impl Step for Count {
    const ENCODE_IN_PLACE: bool = true;
    const ENCODE_UNCHANGED: bool = true;
    const HAS_COUNT: bool = true;

    #[inline(always)]
    fn encoded_len(input_len: usize) -> Result<usize, Error> {
        Ok(input_len)
    }

    #[inline(always)]
    fn counted_len(input_len: usize) -> Result<usize, Error> {
        Ok(input_len)
    }

    #[inline(always)]
    fn encode<'a>(output: &mut &'a mut [u8], _scratch: &mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
        crate::primitive::bytes::copy_bytes(output, input)
    }

    #[inline(always)]
    fn decode<'a>(input: &'a [u8], _scratch: &mut &'a mut [u8], _len: Option<usize>) -> Result<&'a [u8], Error> {
        Ok(input)
    }

    #[inline(always)]
    fn decode_counted<'a>(input: &'a [u8], _scratch: &mut &'a mut [u8], _count: Option<usize>) -> Result<&'a [u8], Error> {
        Ok(input)
    }

    #[inline(always)]
    fn encode_in_place(_buf: &mut [u8], _input_len: usize) -> Result<(), Error> {
        Ok(())
    }
}

impl<First: Step, Rest: Step> Step for Chain<First, Rest> {
    const ENCODE_IN_PLACE: bool = First::ENCODE_IN_PLACE && Rest::ENCODE_IN_PLACE;
    const ENCODE_APPENDING: bool = First::ENCODE_APPENDING && Rest::ENCODE_APPENDING;
    const ENCODE_UNCHANGED: bool = First::ENCODE_UNCHANGED && Rest::ENCODE_UNCHANGED;
    const HAS_COUNT: bool = {
        assert!(!(First::HAS_COUNT && Rest::HAS_COUNT), "a step chain has at most one Count marker");
        First::HAS_COUNT || Rest::HAS_COUNT
    };

    #[inline(always)]
    fn encoded_len(input_len: usize) -> Result<usize, Error> {
        Rest::encoded_len(First::encoded_len(input_len)?)
    }

    #[inline(always)]
    fn counted_len(input_len: usize) -> Result<usize, Error> {
        if First::HAS_COUNT {
            First::counted_len(input_len)
        } else {
            Rest::counted_len(First::encoded_len(input_len)?)
        }
    }

    #[inline(always)]
    fn counted_wire_len(count: usize) -> Result<usize, Error> {
        if First::HAS_COUNT {
            Rest::encoded_len(First::counted_wire_len(count)?)
        } else {
            Rest::counted_wire_len(count)
        }
    }

    #[inline(always)]
    fn encode<'a>(output: &mut &'a mut [u8], scratch: &mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
        if First::ENCODE_UNCHANGED {
            return Rest::encode(output, scratch, input);
        }
        if Rest::ENCODE_IN_PLACE {
            let buf = First::encode(output, scratch, input)?;
            let len = buf.len();
            Rest::encode_in_place(buf, len)?;
            return Ok(buf);
        }
        if Rest::ENCODE_APPENDING {
            // `First` writes at the start of the output, and `Rest` finishes
            // there. Its written length decides the area: for UTF-8 input to
            // `Ebcdic1142`, `encoded_len` is only a bound.
            let area = core::mem::take(output);
            let written = {
                let mut first_out = &mut *area;
                First::encode(&mut first_out, scratch, input)?.len()
            };
            let Some((buf, rest)) = area.split_at_mut_checked(Rest::encoded_len(written)?) else {
                cold_path();
                return Err(Error::BufferOverflow);
            };
            *output = rest;
            Rest::encode_in_place(buf, written)?;
            return Ok(buf);
        }
        let mid_len = First::encoded_len(input.len())?;
        let (mid_buf, scratch) = split_scratch(scratch, mid_len)?;
        let mid = First::encode(&mut &mut *mid_buf, scratch, input)?;
        Rest::encode(output, scratch, mid)
    }

    #[inline(always)]
    fn decode<'a>(input: &'a [u8], scratch: &mut &'a mut [u8], len: Option<usize>) -> Result<&'a [u8], Error> {
        let mid_len = match len {
            Some(len) => Some(First::encoded_len(len)?),
            None => None,
        };
        let mid = Rest::decode(input, scratch, mid_len)?;
        First::decode(mid, scratch, len)
    }

    #[inline(always)]
    fn decode_counted<'a>(input: &'a [u8], scratch: &mut &'a mut [u8], count: Option<usize>) -> Result<&'a [u8], Error> {
        if First::HAS_COUNT {
            let mid_len = match count {
                Some(count) => Some(First::counted_wire_len(count)?),
                None => None,
            };
            let mid = Rest::decode(input, scratch, mid_len)?;
            First::decode_counted(mid, scratch, count)
        } else {
            let mid = Rest::decode_counted(input, scratch, count)?;
            First::decode(mid, scratch, None)
        }
    }

    #[inline(always)]
    fn encode_in_place(buf: &mut [u8], input_len: usize) -> Result<(), Error> {
        let mid_len = First::encoded_len(input_len)?;
        let area = buf.get_mut(..mid_len);
        debug_assert!(area.is_some(), "step output is shorter than its input");
        First::encode_in_place(area.unwrap_or_default(), input_len)?;
        Rest::encode_in_place(buf, mid_len)
    }
}

impl<S: Step, C: Check> Step for DecodeCheck<S, C> {
    const ENCODE_IN_PLACE: bool = S::ENCODE_IN_PLACE;
    const ENCODE_APPENDING: bool = S::ENCODE_APPENDING;
    const ENCODE_UNCHANGED: bool = S::ENCODE_UNCHANGED;
    const HAS_COUNT: bool = S::HAS_COUNT;

    #[inline(always)]
    fn encoded_len(input_len: usize) -> Result<usize, Error> {
        S::encoded_len(input_len)
    }

    #[inline(always)]
    fn counted_len(input_len: usize) -> Result<usize, Error> {
        S::counted_len(input_len)
    }

    #[inline(always)]
    fn counted_wire_len(count: usize) -> Result<usize, Error> {
        S::counted_wire_len(count)
    }

    #[inline(always)]
    fn encode<'a>(output: &mut &'a mut [u8], scratch: &mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
        S::encode(output, scratch, input)
    }

    #[inline(always)]
    fn decode<'a>(input: &'a [u8], scratch: &mut &'a mut [u8], len: Option<usize>) -> Result<&'a [u8], Error> {
        C::validate(input).map_err(length_as_invalid)?;
        S::decode(input, scratch, len)
    }

    #[inline(always)]
    fn decode_counted<'a>(input: &'a [u8], scratch: &mut &'a mut [u8], count: Option<usize>) -> Result<&'a [u8], Error> {
        C::validate(input).map_err(length_as_invalid)?;
        S::decode_counted(input, scratch, count)
    }

    #[inline(always)]
    fn encode_in_place(buf: &mut [u8], input_len: usize) -> Result<(), Error> {
        S::encode_in_place(buf, input_len)
    }
}

#[cfg(test)]
mod tests {
    use super::Chain;
    use crate::primitive::nibble::UpperHexDigits;
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
        encode_without_scratch::<Field<Ascii<0, 8>, Fixed<8>, Chain<PadRight<8>, Ebcdic037>>>(
            b"ABC",
            &[0xC1, 0xC2, 0xC3, 0x40, 0x40, 0x40, 0x40, 0x40],
        );
        // Padding after a transform fills around what the transform wrote.
        encode_without_scratch::<Field<Ascii<3, 3>, Fixed<3>, crate::chain!(Ebcdic037, Count, PadRight<5>)>>(b"ABC", b"\xC1\xC2\xC3  ");
        encode_without_scratch::<Field<Ebcdic1142Text<0, 3>, Fixed<4>, crate::chain!(Ebcdic1142, PadRight<4, 0x40>)>>(
            "Æ".as_bytes(),
            b"\x7B\x40\x40\x40",
        );
        encode_without_scratch::<Field<Binary<0, 2>, Fixed<5>, crate::chain!(UnpackNibbles<UpperHexDigits>, PadRight<5, b'0'>)>>(
            b"\x0A", b"0A000",
        );
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
}

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    use crate::primitive::nibble::UpperHexDigits;
    use crate::{Binary, Field, PadRight, Rest, ScalarFmt, UnpackNibbles};

    proptest! {
        #[test]
        fn padded_hex_roundtrip(value in prop::collection::vec(any::<u8>(), 0..=32)) {
            type F = Field<Binary<0,32>, Rest, crate::chain!(UnpackNibbles<UpperHexDigits>, PadRight<3>)>;
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
