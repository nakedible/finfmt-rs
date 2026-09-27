use super::*;
use crate::primitive::bytes::{copy_bytes, is_filled, reserve_filled, take_bytes};

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
    pub trait TrailingTailSealed {}
    pub trait TrailingTailsSealed {}
}

#[doc(hidden)]
pub trait TrailingTail: trailing_sealed::TrailingTailSealed {
    const WIRE_LEN: usize;

    fn is_absent(input: &[u8], scratch: &mut &mut [u8]) -> Result<bool, Error>;
}

#[doc(hidden)]
pub trait TrailingTails: trailing_sealed::TrailingTailsSealed {
    const WIRE_LEN: usize;

    fn trim_len(input: &[u8], scratch: &mut &mut [u8]) -> Result<usize, Error>;
    fn is_boundary(len: usize) -> bool;
    fn validate_omitted(input: &[u8], included_len: usize, scratch: &mut &mut [u8]) -> Result<(), Error>;
}

impl<Inner, Absent: AbsentFmt, const N: usize> trailing_sealed::TrailingTailSealed for OptionalAbsent<Inner, Absent, N> {}

impl<Inner, Absent: AbsentFmt, const N: usize> TrailingTail for OptionalAbsent<Inner, Absent, N> {
    const WIRE_LEN: usize = N;

    #[inline(always)]
    fn is_absent(input: &[u8], scratch: &mut &mut [u8]) -> Result<bool, Error> {
        const { assert!(N != 0, "an OptionalAbsent area must be at least one byte wide") };
        debug_assert_eq!(input.len(), N, "a trailing field was given a different width");
        Absent::is_absent(input, scratch)
    }
}

impl trailing_sealed::TrailingTailsSealed for NoTrailingFields {}

impl TrailingTails for NoTrailingFields {
    const WIRE_LEN: usize = 0;

    #[inline(always)]
    fn trim_len(input: &[u8], _scratch: &mut &mut [u8]) -> Result<usize, Error> {
        debug_assert!(input.is_empty(), "bytes left after the last trailing field");
        Ok(0)
    }

    #[inline(always)]
    fn is_boundary(len: usize) -> bool {
        len == 0
    }

    #[inline(always)]
    fn validate_omitted(input: &[u8], included_len: usize, _scratch: &mut &mut [u8]) -> Result<(), Error> {
        debug_assert!(input.is_empty() && included_len == 0, "bytes left after the last trailing field");
        Ok(())
    }
}

impl<Field, Rest> trailing_sealed::TrailingTailsSealed for TrailingField<Field, Rest>
where
    Field: TrailingTail,
    Rest: TrailingTails,
{
}

impl<Field, Rest> TrailingTails for TrailingField<Field, Rest>
where
    Field: TrailingTail,
    Rest: TrailingTails,
{
    const WIRE_LEN: usize = Field::WIRE_LEN + Rest::WIRE_LEN;

    #[inline(always)]
    fn trim_len(input: &[u8], scratch: &mut &mut [u8]) -> Result<usize, Error> {
        debug_assert_eq!(input.len(), Self::WIRE_LEN, "trailing fields were given a different width");
        let (head, rest) = input.split_at_checked(Field::WIRE_LEN).unwrap_or((input, &[]));
        let rest_len = Rest::trim_len(rest, scratch)?;
        if rest_len != 0 {
            return Field::WIRE_LEN.checked_add(rest_len).ok_or_else(|| {
                crate::utils::cold_path();
                Error::BufferOverflow
            });
        }
        if Field::is_absent(head, scratch)? {
            Ok(0)
        } else {
            Ok(Field::WIRE_LEN)
        }
    }

    #[inline(always)]
    fn is_boundary(len: usize) -> bool {
        Field::WIRE_LEN != 0 && (len == 0 || len >= Field::WIRE_LEN && Rest::is_boundary(len - Field::WIRE_LEN))
    }

    #[inline(always)]
    fn validate_omitted(input: &[u8], included_len: usize, scratch: &mut &mut [u8]) -> Result<(), Error> {
        // Decoding checked `is_boundary` first, so the included length fits.
        debug_assert!(
            input.len() == Self::WIRE_LEN && included_len <= Self::WIRE_LEN,
            "trailing fields were given a different width"
        );
        let (head, rest) = input.split_at_checked(Field::WIRE_LEN).unwrap_or((input, &[]));
        if included_len == 0 {
            if !Field::is_absent(head, scratch)? {
                crate::utils::cold_path();
                return Err(Error::Invalid);
            }
            return Rest::validate_omitted(rest, 0, scratch);
        }
        if included_len < Field::WIRE_LEN {
            crate::utils::cold_path();
            return Err(Error::Invalid);
        }
        Rest::validate_omitted(rest, included_len - Field::WIRE_LEN, scratch)
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
        let tail_len = Tails::trim_len(tails, &mut &mut *scratch)?;
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

impl<T, Inner: FieldEncode<T>, Absent: AbsentFmt, const N: usize> FieldEncode<Option<T>> for OptionalAbsent<Inner, Absent, N> {
    #[inline(always)]
    fn encode_field(output: &mut &mut [u8], scratch: &mut [u8], value: &Option<T>) -> Result<(), CompositeError> {
        const { assert!(N != 0, "an OptionalAbsent area must be at least one byte wide") };
        let available = output.len();
        match value {
            None => Absent::encode_absent(output, scratch, N)?,
            Some(value) => Inner::encode_field(output, scratch, value)?,
        }
        debug_assert_eq!(available - output.len(), N, "the value is not as wide as its OptionalAbsent area");
        Ok(())
    }
}

impl<'de, T, Inner: FieldDecode<'de, T>, Absent: AbsentFmt, const N: usize> FieldDecode<'de, Option<T>>
    for OptionalAbsent<Inner, Absent, N>
{
    #[inline(always)]
    fn decode_field(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Option<T>, CompositeError> {
        const { assert!(N != 0, "an OptionalAbsent area must be at least one byte wide") };
        let area = take_bytes(input, N)?;
        if Absent::is_absent(area, scratch)? {
            return Ok(None);
        }
        let mut area_input = area;
        let value = Inner::decode_field(&mut area_input, scratch)?;
        if !area_input.is_empty() {
            crate::utils::cold_path();
            return Err(Error::Invalid.into());
        }
        Ok(Some(value))
    }
}

impl<const BYTE: u8> AbsentFmt for ByteFill<BYTE> {
    #[inline(always)]
    fn encode_absent(output: &mut &mut [u8], _scratch: &mut [u8], len: usize) -> Result<(), Error> {
        reserve_filled(output, len, BYTE)?;
        Ok(())
    }

    #[inline(always)]
    fn is_absent(input: &[u8], _scratch: &mut &mut [u8]) -> Result<bool, Error> {
        Ok(is_filled(input, BYTE))
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
    use crate::{Ascii, Ebcdic037, Field, Fixed};

    type A3 = Field<Ascii<3, 3>, Fixed<3>>;
    crate::absent_format! {
        struct Dashes {
            _: A3 = b"---",
        }
    }

    #[test]
    fn a_value_narrower_than_its_area_writes_only_its_own_bytes() {
        type Short = OptionalAbsent<Field<Ascii<1, 3>, crate::Rest>, ByteFill, 3>;
        let result = std::panic::catch_unwind(|| {
            let mut output = [0xEE; 4];
            let mut out = &mut output[..];
            Short::encode_field(&mut out, &mut [0; 8], &Some(String::from("AB"))).unwrap();
            let left = out.len();
            (output, left)
        });
        if cfg!(debug_assertions) {
            assert!(result.is_err());
        } else {
            // The area is not reserved up front, so no stale byte is left inside it.
            assert_eq!(result.unwrap(), (*b"AB\xEE\xEE", 2));
        }
    }

    #[test]
    fn absence_comparison_reuses_scratch_before_borrowed_decode() {
        for (wire, capacity, expected) in [
            (b"---", 3, Ok(true)),
            (b"ABC", 3, Ok(false)),
            (b"---", 32, Ok(true)),
            (b"---", 2, Err(Error::BufferOverflow)),
        ] {
            let mut scratch = [0; 32];
            let mut workspace = &mut scratch[..capacity];
            let start = workspace.as_ptr();
            assert_eq!(Dashes::is_absent(wire, &mut workspace), expected);
            assert_eq!(workspace.len(), capacity);
            assert_eq!(workspace.as_ptr(), start);
        }

        type Text = Field<Ascii<3, 3>, Fixed<3>, Ebcdic037>;
        type OptionalText = OptionalAbsent<Text, Dashes, 3>;
        for capacity in [3, 32] {
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
