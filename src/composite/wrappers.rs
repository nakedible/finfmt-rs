use super::*;
use crate::primitive::bytes::{all_bytes_eq, copy_bytes, fill_tail};

impl<T, F: ScalarFmt, S: CompositeFmt<T>> CompositeFmt<T> for Frame<F, S> {
    type Decoded<'de> = S::Decoded<'de>;

    #[inline(always)]
    fn encode_cursor(output: &mut &mut [u8], scratch: &mut &mut [u8], value: &T) -> Result<(), StructError> {
        let scratch_len = scratch.len();
        let used = {
            let mut semantic_out = &mut **scratch;
            let mut nested_scratch = &mut output[..];
            S::encode_cursor(&mut semantic_out, &mut nested_scratch, value)?;
            scratch_len - semantic_out.len()
        };
        let mut scratch_tail = core::mem::take(scratch);
        let semantic = scratch_tail.split_off_mut(..used).ok_or_else(|| {
            crate::utils::cold_path();
            StructError::from(Error::BufferOverflow)
        })?;
        F::encode(output, &mut scratch_tail, semantic)?;
        *scratch = scratch_tail;
        Ok(())
    }

    #[inline(always)]
    fn decode_cursor<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<Self::Decoded<'a>, StructError> {
        let source = *input;
        let mut input_ptr = source;
        let value_bytes = F::decode(&mut input_ptr, scratch)?;
        advance_input(input, source.len() - input_ptr.len())?;

        let mut value_input = value_bytes;
        let value = S::decode_cursor(&mut value_input, scratch)?;
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

impl<T, Inner, Absent, const N: usize> trailing_sealed::TrailingTailSealed for OptionalAbsent<T, Inner, Absent, N>
where
    Inner: CompositeFmt<T>,
    Absent: AbsentFmt,
{
}

impl<T, Inner, Absent, const N: usize> TrailingTail for OptionalAbsent<T, Inner, Absent, N>
where
    Inner: CompositeFmt<T>,
    Absent: AbsentFmt,
{
    const WIRE_LEN: usize = N;

    #[inline(always)]
    fn is_absent(input: &[u8], scratch: &mut &mut [u8]) -> Result<bool, Error> {
        if input.len() != Self::WIRE_LEN || Self::WIRE_LEN == 0 {
            crate::utils::cold_path();
            return Err(Error::Internal);
        }
        Absent::is_absent(input, scratch)
    }
}

impl trailing_sealed::TrailingTailsSealed for NoTrailingFields {}

impl TrailingTails for NoTrailingFields {
    const WIRE_LEN: usize = 0;

    #[inline(always)]
    fn trim_len(input: &[u8], _scratch: &mut &mut [u8]) -> Result<usize, Error> {
        if !input.is_empty() {
            crate::utils::cold_path();
            return Err(Error::Internal);
        }
        Ok(0)
    }

    #[inline(always)]
    fn is_boundary(len: usize) -> bool {
        len == 0
    }

    #[inline(always)]
    fn validate_omitted(input: &[u8], included_len: usize, _scratch: &mut &mut [u8]) -> Result<(), Error> {
        if input.is_empty() && included_len == 0 {
            return Ok(());
        }
        crate::utils::cold_path();
        Err(Error::Internal)
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
        if Field::WIRE_LEN == 0 || input.len() != Self::WIRE_LEN {
            crate::utils::cold_path();
            return Err(Error::Internal);
        }
        let head = input.get(..Field::WIRE_LEN).ok_or_else(|| {
            crate::utils::cold_path();
            Error::Internal
        })?;
        let rest = input.get(Field::WIRE_LEN..).ok_or_else(|| {
            crate::utils::cold_path();
            Error::Internal
        })?;
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
        if Field::WIRE_LEN == 0 || input.len() != Self::WIRE_LEN || included_len > Self::WIRE_LEN {
            crate::utils::cold_path();
            return Err(Error::Internal);
        }
        let head = input.get(..Field::WIRE_LEN).ok_or_else(|| {
            crate::utils::cold_path();
            Error::Internal
        })?;
        let rest = input.get(Field::WIRE_LEN..).ok_or_else(|| {
            crate::utils::cold_path();
            Error::Internal
        })?;
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

impl<T, Len, Body, Tails, const BASE_LEN: usize> CompositeFmt<T> for TrailingLengthFrame<T, Len, Body, Tails, BASE_LEN>
where
    Len: crate::field::LengthSpec<crate::field::Identity>,
    Body: CompositeFmt<T>,
    Tails: TrailingTails,
{
    type Decoded<'de> = Body::Decoded<'de>;

    #[inline(always)]
    fn encode_cursor(output: &mut &mut [u8], scratch: &mut &mut [u8], value: &T) -> Result<(), StructError> {
        let full_len = trailing_body_len::<Tails, BASE_LEN>()?;
        let mut scratch_tail = core::mem::take(scratch);
        let body = take_scratch(&mut scratch_tail, full_len)?;
        let mut body_out = &mut body[..];
        Body::encode_cursor(&mut body_out, &mut scratch_tail, value)?;
        if !body_out.is_empty() {
            crate::utils::cold_path();
            return Err(Error::Internal.into());
        }
        let tails = body.get(BASE_LEN..).ok_or_else(|| {
            crate::utils::cold_path();
            StructError::from(Error::Internal)
        })?;
        let tail_len = Tails::trim_len(tails, &mut scratch_tail)?;
        let logical_len = BASE_LEN.checked_add(tail_len).ok_or_else(|| {
            crate::utils::cold_path();
            StructError::from(Error::BufferOverflow)
        })?;
        Len::encode(output, &mut scratch_tail, logical_len, logical_len)?;
        copy_bytes(output, body)?;
        *scratch = scratch_tail;
        Ok(())
    }

    #[inline(always)]
    fn decode_cursor<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<Self::Decoded<'a>, StructError> {
        let plan = Len::decode_plan(input, scratch)?;
        let logical_len = plan.exact_len.ok_or_else(|| {
            crate::utils::cold_path();
            StructError::from(Error::Internal)
        })?;
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
        let body = input.split_off(..full_len).ok_or_else(|| {
            crate::utils::cold_path();
            StructError::from(Error::UnexpectedEof)
        })?;
        let tails = body.get(BASE_LEN..).ok_or_else(|| {
            crate::utils::cold_path();
            StructError::from(Error::Internal)
        })?;
        Tails::validate_omitted(tails, tail_len, scratch)?;

        let mut body_input = body;
        let value = Body::decode_cursor(&mut body_input, scratch)?;
        if !body_input.is_empty() {
            crate::utils::cold_path();
            return Err(Error::Invalid.into());
        }
        Ok(value)
    }
}

impl<T, Inner, Absent, const N: usize> CompositeFmt<Option<T>> for OptionalAbsent<T, Inner, Absent, N>
where
    Inner: CompositeFmt<T>,
    Absent: AbsentFmt,
{
    type Decoded<'de> = Option<Inner::Decoded<'de>>;

    #[inline(always)]
    fn encode_cursor(output: &mut &mut [u8], scratch: &mut &mut [u8], value: &Option<T>) -> Result<(), StructError> {
        let area = output.split_off_mut(..N).ok_or_else(|| {
            crate::utils::cold_path();
            StructError::from(Error::BufferOverflow)
        })?;
        let mut area_out = &mut area[..];
        match value {
            None => Absent::encode_absent(&mut area_out, scratch)?,
            Some(value) => Inner::encode_cursor(&mut area_out, scratch, value)?,
        }
        if !area_out.is_empty() {
            crate::utils::cold_path();
            return Err(Error::Internal.into());
        }
        Ok(())
    }

    #[inline(always)]
    fn decode_cursor<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<Self::Decoded<'a>, StructError> {
        let area = input.split_off(..N).ok_or_else(|| {
            crate::utils::cold_path();
            StructError::from(Error::UnexpectedEof)
        })?;
        if Absent::is_absent(area, scratch)? {
            return Ok(None);
        }
        let mut area_input = area;
        let value = Inner::decode_cursor(&mut area_input, scratch)?;
        if !area_input.is_empty() {
            crate::utils::cold_path();
            return Err(Error::Invalid.into());
        }
        Ok(Some(value))
    }
}

impl<const BYTE: u8> AbsentFmt for ByteFill<BYTE> {
    #[inline(always)]
    fn encode_absent(output: &mut &mut [u8], _scratch: &mut &mut [u8]) -> Result<(), Error> {
        let area = core::mem::take(output);
        fill_tail(area, 0, BYTE)?;
        Ok(())
    }

    #[inline(always)]
    fn is_absent(input: &[u8], _scratch: &mut &mut [u8]) -> Result<bool, Error> {
        Ok(all_bytes_eq(input, BYTE))
    }
}

impl<T: Default> CompositeFmt<T> for Empty<T> {
    type Decoded<'de> = T;

    #[inline(always)]
    fn encode_cursor(_output: &mut &mut [u8], _scratch: &mut &mut [u8], _value: &T) -> Result<(), StructError> {
        Ok(())
    }

    #[inline(always)]
    fn decode_cursor<'a>(_input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<Self::Decoded<'a>, StructError> {
        Ok(T::default())
    }
}
