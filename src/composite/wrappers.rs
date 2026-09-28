use super::*;
use crate::primitive::bytes::{copy_bytes, take_bytes};

impl<T: ?Sized, F: ScalarFmt, S: FieldEncode<T>> FieldEncode<T> for Frame<F, S> {
    #[inline(always)]
    fn encode_field(output: &mut &mut [u8], scratch: &mut [u8], value: &T) -> Result<(), CompositeError> {
        // The inner value is staged in scratch, using the unwritten output as its workspace.
        let used = {
            let mut semantic_out = &mut *scratch;
            let available = semantic_out.len();
            S::encode_field(&mut semantic_out, output, value)?;
            available - semantic_out.len()
        };
        let (semantic, scratch) = split_scratch(scratch, used)?;
        F::encode(output, scratch, semantic)?;
        Ok(())
    }
}

impl<'de, T, F: ScalarFmt, S: FieldDecode<'de, T>> FieldDecode<'de, T> for Frame<F, S> {
    const TAKES_REST: bool = F::TAKES_REST;

    #[inline(always)]
    fn decode_field(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<T, CompositeError> {
        let source = *input;
        let mut input_ptr = source;
        let value_bytes = F::decode(&mut input_ptr, scratch)?;
        advance_input(input, source.len() - input_ptr.len())?;

        let mut value_input = value_bytes;
        let value = S::decode_field(&mut value_input, scratch)?;
        if !value_input.is_empty() {
            crate::utils::cold_path();
            return Err(Error::Invalid.into());
        }
        Ok(value)
    }
}

mod trailing_sealed {
    pub trait TrailingTailsSealed {}
}

#[doc(hidden)]
pub trait TrailingTails: trailing_sealed::TrailingTailsSealed {
    const WIRE_LEN: usize;

    fn trim_len(input: &[u8], scratch: &mut [u8]) -> Result<usize, Error>;
    fn is_boundary(len: usize) -> bool;
    fn validate_omitted(input: &[u8], included_len: usize, scratch: &mut [u8]) -> Result<(), Error>;
}

/// Whether `slot` is exactly one absent encoding.
#[inline(always)]
fn slot_is_absent<Absent: AbsentFmt>(slot: &[u8], scratch: &mut [u8]) -> Result<bool, Error> {
    let mut rest = slot;
    Ok(Absent::decode_absent(&mut rest, scratch)? && rest.is_empty())
}

impl trailing_sealed::TrailingTailsSealed for NoTrailingFields {}

impl TrailingTails for NoTrailingFields {
    const WIRE_LEN: usize = 0;

    #[inline(always)]
    fn trim_len(input: &[u8], _scratch: &mut [u8]) -> Result<usize, Error> {
        debug_assert!(input.is_empty(), "bytes left after the last trailing field");
        Ok(0)
    }

    #[inline(always)]
    fn is_boundary(len: usize) -> bool {
        len == 0
    }

    #[inline(always)]
    fn validate_omitted(input: &[u8], included_len: usize, _scratch: &mut [u8]) -> Result<(), Error> {
        debug_assert!(input.is_empty() && included_len == 0, "bytes left after the last trailing field");
        Ok(())
    }
}

impl<Absent: AbsentFmt, const WIDTH: usize, Rest: TrailingTails> trailing_sealed::TrailingTailsSealed
    for TrailingField<Absent, WIDTH, Rest>
{
}

impl<Absent: AbsentFmt, const WIDTH: usize, Rest: TrailingTails> TrailingTails for TrailingField<Absent, WIDTH, Rest> {
    const WIRE_LEN: usize = {
        assert!(WIDTH != 0, "a trailing field must be at least one byte wide");
        WIDTH + Rest::WIRE_LEN
    };

    #[inline(always)]
    fn trim_len(input: &[u8], scratch: &mut [u8]) -> Result<usize, Error> {
        debug_assert_eq!(input.len(), Self::WIRE_LEN, "trailing fields were given a different width");
        let (head, rest) = input.split_at_checked(WIDTH).unwrap_or((input, &[]));
        let rest_len = Rest::trim_len(rest, scratch)?;
        if rest_len != 0 {
            return WIDTH.checked_add(rest_len).ok_or_else(|| {
                crate::utils::cold_path();
                Error::BufferOverflow
            });
        }
        if slot_is_absent::<Absent>(head, scratch)? {
            Ok(0)
        } else {
            Ok(WIDTH)
        }
    }

    #[inline(always)]
    fn is_boundary(len: usize) -> bool {
        len == 0 || len >= WIDTH && Rest::is_boundary(len - WIDTH)
    }

    #[inline(always)]
    fn validate_omitted(input: &[u8], included_len: usize, scratch: &mut [u8]) -> Result<(), Error> {
        // Decoding checked `is_boundary` first, so the included length fits.
        debug_assert!(
            input.len() == Self::WIRE_LEN && included_len <= Self::WIRE_LEN,
            "trailing fields were given a different width"
        );
        let (head, rest) = input.split_at_checked(WIDTH).unwrap_or((input, &[]));
        if included_len == 0 {
            if !slot_is_absent::<Absent>(head, scratch)? {
                crate::utils::cold_path();
                return Err(Error::Invalid);
            }
            return Rest::validate_omitted(rest, 0, scratch);
        }
        if included_len < WIDTH {
            crate::utils::cold_path();
            return Err(Error::Invalid);
        }
        Rest::validate_omitted(rest, included_len - WIDTH, scratch)
    }
}

#[inline(always)]
fn trailing_body_len<Tails: TrailingTails, const BASE_LEN: usize>() -> Result<usize, Error> {
    BASE_LEN.checked_add(Tails::WIRE_LEN).ok_or_else(|| {
        crate::utils::cold_path();
        Error::BufferOverflow
    })
}

impl<T: ?Sized, Len, Body, Tails, const BASE_LEN: usize> FieldEncode<T> for TrailingLengthFrame<Len, Body, Tails, BASE_LEN>
where
    Len: crate::field::LengthSpec,
    Body: FieldEncode<T>,
    Tails: TrailingTails,
{
    #[inline(always)]
    fn encode_field(output: &mut &mut [u8], scratch: &mut [u8], value: &T) -> Result<(), CompositeError> {
        let full_len = trailing_body_len::<Tails, BASE_LEN>()?;
        let (staging, scratch) = split_scratch(scratch, full_len)?;
        let mut body_out = &mut staging[..];
        Body::encode_field(&mut body_out, scratch, value)?;
        // Copy only what the body wrote, so a miswritten body never ships stale scratch.
        let written = full_len - body_out.len();
        debug_assert_eq!(written, full_len, "the body is not as wide as its declared length");
        let body = staging.get(..written).unwrap_or_default();
        let tails = body.get(BASE_LEN..).unwrap_or_default();
        let tail_len = Tails::trim_len(tails, scratch)?;
        let logical_len = BASE_LEN.checked_add(tail_len).ok_or_else(|| {
            crate::utils::cold_path();
            CompositeError::from(Error::BufferOverflow)
        })?;
        Len::encode(output, scratch, logical_len)?;
        copy_bytes(output, body)?;
        Ok(())
    }
}

impl<'de, T, Len, Body, Tails, const BASE_LEN: usize> FieldDecode<'de, T> for TrailingLengthFrame<Len, Body, Tails, BASE_LEN>
where
    Len: crate::field::LengthSpec,
    Body: FieldDecode<'de, T>,
    Tails: TrailingTails,
{
    #[inline(always)]
    fn decode_field(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<T, CompositeError> {
        const { assert!(Len::STATES_LEN, "a trailing-length frame needs a declared length") };
        let logical_len = Len::decode(input, scratch)?.unwrap_or_default();
        if logical_len < BASE_LEN {
            crate::utils::cold_path();
            return Err(Error::Invalid.into());
        }
        let tail_len = logical_len - BASE_LEN;
        if !Tails::is_boundary(tail_len) {
            crate::utils::cold_path();
            return Err(Error::Invalid.into());
        }

        let full_len = trailing_body_len::<Tails, BASE_LEN>()?;
        let body = take_bytes(input, full_len)?;
        let tails = body.get(BASE_LEN..).unwrap_or_default();
        Tails::validate_omitted(tails, tail_len, scratch)?;

        let mut body_input = body;
        let value = Body::decode_field(&mut body_input, scratch)?;
        if !body_input.is_empty() {
            crate::utils::cold_path();
            return Err(Error::Invalid.into());
        }
        Ok(value)
    }
}

impl<T, Inner: FieldEncode<T>, Absent: AbsentFmt> FieldEncode<Option<T>> for OptionAs<Inner, Absent> {
    #[inline(always)]
    fn encode_field(output: &mut &mut [u8], scratch: &mut [u8], value: &Option<T>) -> Result<(), CompositeError> {
        match value {
            None => Absent::encode_absent(output, scratch)?,
            Some(value) => Inner::encode_field(output, scratch, value)?,
        }
        Ok(())
    }
}

impl<'de, T, Inner: FieldDecode<'de, T>, Absent: AbsentFmt> FieldDecode<'de, Option<T>> for OptionAs<Inner, Absent> {
    const TAKES_REST: bool = Inner::TAKES_REST;

    #[inline(always)]
    fn decode_field(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Option<T>, CompositeError> {
        let mut rest = *input;
        if Absent::decode_absent(&mut rest, scratch)? && (!Inner::TAKES_REST || rest.is_empty()) {
            advance_input(input, input.len() - rest.len())?;
            return Ok(None);
        }
        Ok(Some(Inner::decode_field(input, scratch)?))
    }
}

impl<T: ?Sized> FieldEncode<T> for Empty {
    #[inline(always)]
    fn encode_field(_output: &mut &mut [u8], _scratch: &mut [u8], _value: &T) -> Result<(), CompositeError> {
        Ok(())
    }
}

impl<T: Default> FieldDecode<'_, T> for Empty {
    #[inline(always)]
    fn decode_field(_input: &mut &[u8], _scratch: &mut &mut [u8]) -> Result<T, CompositeError> {
        Ok(T::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Ascii, AsciiLength, Ebcdic037, Field, Fill, Fixed, Numeric, PadLeft, Rest};

    type A3 = Field<Ascii<3, 3>, Fixed<3>>;

    /// An absent encoding given as a value through a format: the default
    /// `decode_absent` encodes and compares it.
    struct Dashes;
    impl AbsentFmt for Dashes {
        fn encode_absent(output: &mut &mut [u8], scratch: &mut [u8]) -> Result<(), Error> {
            <A3 as FieldEncode<str>>::encode_field(output, scratch, "---").map_err(|error| error.kind)
        }
    }

    fn roundtrip<F>(value: Option<&str>, wire: &[u8])
    where
        F: for<'a> FieldEncode<Option<&'a str>> + for<'de> FieldDecode<'de, Option<&'de str>>,
    {
        let mut output = [0; 16];
        let used = crate::encode::<F, _>(&mut output, &mut [0; 16], &value).unwrap();
        assert_eq!(&output[..used], wire);
        assert_eq!(crate::decode::<F, Option<&str>>(wire, &mut [0; 16]), Ok(value));
    }

    #[test]
    fn absent_patterns_are_matched_before_the_value_format() {
        // Blanks a numeric field cannot represent.
        type Amount = OptionAs<Field<Numeric<1, 3>, Fixed<3>, PadLeft<3, b'0', 1>>, AbsentBytes<Fill<b' ', 3>>>;
        roundtrip::<Amount>(None, b"   ");
        roundtrip::<Amount>(Some("12"), b"012");
        // Anything else is decoded by the value's format, whose errors are returned.
        assert_eq!(
            crate::decode::<Amount, Option<&str>>(b"1 2", &mut []).map_err(|error| error.kind),
            Err(Error::Invalid)
        );
        // A length-prefixed field's absent encoding is a complete encoding.
        type Prefixed = OptionAs<Field<Ascii<1, 9>, AsciiLength<1>>, AbsentBytes<Fill<b'0', 1>>>;
        roundtrip::<Prefixed>(None, b"0");
        roundtrip::<Prefixed>(Some("AB"), b"2AB");
    }

    #[test]
    fn a_valid_value_as_the_pattern_reads_back_as_absent() {
        struct Zero;
        impl AbsentFmt for Zero {
            fn encode_absent(output: &mut &mut [u8], scratch: &mut [u8]) -> Result<(), Error> {
                <A3 as FieldEncode<str>>::encode_field(output, scratch, "000").map_err(|error| error.kind)
            }
        }
        type Count = OptionAs<A3, Zero>;
        let mut output = [0; 3];
        crate::encode::<Count, _>(&mut output, &mut [0; 8], &Some("000")).unwrap();
        assert_eq!(crate::decode::<Count, Option<&str>>(&output, &mut [0; 8]), Ok(None));
    }

    #[test]
    fn rest_fields_match_the_whole_remainder() {
        type Remark = OptionAs<Field<Ascii<0, 9>, Rest>, AbsentBytes<Fill<b'-', 3>>>;
        roundtrip::<Remark>(None, b"---");
        roundtrip::<Remark>(Some("---X"), b"---X");
        type Empty = OptionAs<Field<Ascii<1, 9>, Rest>, AbsentBytes<Fill<b' ', 0>>>;
        roundtrip::<Empty>(None, b"");
        roundtrip::<Empty>(Some("AB"), b"AB");
    }

    #[test]
    fn absence_may_consume_nothing() {
        // Present only when the value's own identifier follows.
        struct UnlessId;
        impl AbsentFmt for UnlessId {
            fn encode_absent(_output: &mut &mut [u8], _scratch: &mut [u8]) -> Result<(), Error> {
                Ok(())
            }
            fn decode_absent(input: &mut &[u8], _scratch: &mut [u8]) -> Result<bool, Error> {
                Ok(!input.starts_with(b"ID"))
            }
        }
        type Tagged = OptionAs<Field<Ascii<4, 4>, Fixed<4>>, UnlessId>;
        let mut input = &b"XYZW"[..];
        assert_eq!(Tagged::decode_field(&mut input, &mut &mut [][..]), Ok(None::<&str>));
        assert_eq!(input, b"XYZW");
        assert_eq!(Tagged::decode_field(&mut input, &mut &mut [][..]), Ok(None::<&str>));
        let mut input = &b"ID12TAIL"[..];
        assert_eq!(Tagged::decode_field(&mut input, &mut &mut [][..]), Ok(Some("ID12")));
        assert_eq!(input, b"TAIL");
    }

    #[test]
    fn absence_comparison_reuses_scratch_before_borrowed_decode() {
        // The default comparison needs twice the pattern's width.
        for (wire, capacity, expected) in [
            (&b"---"[..], 6, Ok(true)),
            (b"ABC", 6, Ok(false)),
            (b"---", 32, Ok(true)),
            (b"---", 5, Err(Error::BufferOverflow)),
        ] {
            let mut scratch = [0; 32];
            let mut input = wire;
            assert_eq!(Dashes::decode_absent(&mut input, &mut scratch[..capacity]), expected);
            assert_eq!(input.is_empty(), expected == Ok(true));
        }

        type Text = Field<Ascii<3, 3>, Fixed<3>, Ebcdic037>;
        type OptionalText = OptionAs<Text, Dashes>;
        for capacity in [6, 32] {
            let mut scratch = [0; 32];
            let start = scratch.as_ptr();
            let mut workspace = &mut scratch[..capacity];
            let mut input = &b"---\xC1\xC2\xC3TAIL"[..];
            assert_eq!(OptionalText::decode_field(&mut input, &mut workspace), Ok(None::<&str>));
            let text: &str = OptionalText::decode_field(&mut input, &mut workspace).unwrap().unwrap();
            assert_eq!(text, "ABC");
            assert_eq!(text.as_ptr(), start);
            assert_eq!(workspace.len(), capacity - 3);
            assert_eq!(input, b"TAIL");
        }
    }
}
