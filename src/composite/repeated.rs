use super::*;
use crate::field::{Identity, LengthSpec};
use crate::primitive::bytes::{contains_byte, fill_repeated_block, split_delimited_bytes, validate_repeating_block};
use crate::utils::take_scratch;

impl ListCountPolicy for () {
    #[inline(always)]
    fn encode_count(_output: &mut &mut [u8], _scratch: &mut &mut [u8], _len: usize) -> Result<(), Error> {
        Ok(())
    }

    #[inline(always)]
    fn decode_count<'a>(_input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<Option<usize>, Error> {
        Ok(None)
    }
}

impl<F: LengthSpec<Identity>> ListCountPolicy for F {
    #[inline(always)]
    fn encode_count(output: &mut &mut [u8], scratch: &mut &mut [u8], len: usize) -> Result<(), Error> {
        F::encode(output, scratch, len, len)
    }

    #[inline(always)]
    fn decode_count<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<Option<usize>, Error> {
        let plan = F::decode_plan(input, scratch)?;
        Ok(Some(plan.exact_len.unwrap_or(plan.output_cap)))
    }
}

impl<const COUNT: usize> ListCountPolicy for FixedCount<COUNT> {
    #[inline(always)]
    fn encode_count(_output: &mut &mut [u8], _scratch: &mut &mut [u8], len: usize) -> Result<(), Error> {
        if len != COUNT {
            crate::utils::cold_path();
            return Err(Error::Invalid);
        }
        Ok(())
    }

    #[inline(always)]
    fn decode_count<'a>(_input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<Option<usize>, Error> {
        Ok(Some(COUNT))
    }
}

impl ListSeparatorPolicy for () {
    const BYTE: Option<u8> = None;
}

impl<const BYTE: u8> ListSeparatorPolicy for Separator<BYTE> {
    const BYTE: Option<u8> = Some(BYTE);
}

#[inline]
fn encode_list_separator<S: ListSeparatorPolicy>(output: &mut &mut [u8]) -> Result<(), Error> {
    if let Some(byte) = S::BYTE {
        encode_delimiter(output, byte)?;
    }
    Ok(())
}

impl<T, Count, Item, Sep, const MAX: usize> CompositeFmt<Vec<T>> for BoundedList<T, Count, Item, Sep, MAX>
where
    Count: ListCountPolicy,
    Item: CompositeFmt<T>,
    Sep: ListSeparatorPolicy,
{
    type Decoded<'de> = Vec<Item::Decoded<'de>>;

    #[inline(always)]
    fn encode_cursor(output: &mut &mut [u8], scratch: &mut &mut [u8], value: &Vec<T>) -> Result<(), StructError> {
        if value.len() > MAX {
            crate::utils::cold_path();
            return Err(Error::Invalid.into());
        }

        Count::encode_count(output, scratch, value.len())?;
        for (index, item) in value.iter().enumerate() {
            if index != 0 {
                encode_list_separator::<Sep>(output)?;
            }
            Item::encode_cursor(output, scratch, item)?;
        }
        Ok(())
    }

    #[inline(always)]
    fn decode_cursor<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<Self::Decoded<'a>, StructError> {
        let expected = Count::decode_count(input, scratch)?;
        let initial_capacity = expected.unwrap_or(0).min(MAX);
        let mut values = Vec::with_capacity(initial_capacity);

        match (expected, Sep::BYTE) {
            (Some(count), Some(separator)) => {
                if count > MAX {
                    crate::utils::cold_path();
                    return Err(Error::Invalid.into());
                }
                for index in 0..count {
                    let mut segment = split_delimited_bytes(input, separator, index + 1 != count)?;
                    let value = Item::decode_cursor(&mut segment, scratch)?;
                    if !segment.is_empty() {
                        crate::utils::cold_path();
                        return Err(Error::Invalid.into());
                    }
                    values.push(value);
                }
            }
            (None, Some(separator)) => {
                while !input.is_empty() {
                    if values.len() == MAX {
                        crate::utils::cold_path();
                        return Err(Error::Invalid.into());
                    }
                    let is_last = !contains_byte(input, separator);
                    let mut segment = split_delimited_bytes(input, separator, !is_last)?;
                    let value = Item::decode_cursor(&mut segment, scratch)?;
                    if !segment.is_empty() {
                        crate::utils::cold_path();
                        return Err(Error::Invalid.into());
                    }
                    values.push(value);
                }
            }
            (Some(count), None) => {
                if count > MAX {
                    crate::utils::cold_path();
                    return Err(Error::Invalid.into());
                }
                for _ in 0..count {
                    let before = input.len();
                    let value = Item::decode_cursor(input, scratch)?;
                    if input.len() == before {
                        crate::utils::cold_path();
                        return Err(Error::Internal.into());
                    }
                    values.push(value);
                }
            }
            (None, None) => {
                while !input.is_empty() {
                    if values.len() == MAX {
                        crate::utils::cold_path();
                        return Err(Error::Invalid.into());
                    }
                    let before = input.len();
                    let value = Item::decode_cursor(input, scratch)?;
                    if input.len() == before {
                        crate::utils::cold_path();
                        return Err(Error::Internal.into());
                    }
                    values.push(value);
                }
            }
        }

        Ok(values)
    }
}

mod sealed {
    pub trait FixedAreaSlotSealed {}
}

#[doc(hidden)]
pub trait FixedAreaSlot<T>: sealed::FixedAreaSlotSealed {
    type Decoded<'de>;

    const WIRE_LEN: usize;

    fn encode_present(output: &mut [u8], scratch: &mut &mut [u8], value: &T) -> Result<(), StructError>;
    fn decode_present<'de>(input: &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self::Decoded<'de>, StructError>;
    fn encode_absent_slots(output: &mut [u8], scratch: &mut &mut [u8]) -> Result<(), StructError>;
    fn validate_absent_slots(input: &[u8], scratch: &mut &mut [u8]) -> Result<(), StructError>;
}

impl<T, Inner, Absent, const N: usize> sealed::FixedAreaSlotSealed for OptionalAbsent<T, Inner, Absent, N>
where
    Inner: CompositeFmt<T>,
    Absent: AbsentFmt,
{
}

impl<T, Inner, Absent, const N: usize> FixedAreaSlot<T> for OptionalAbsent<T, Inner, Absent, N>
where
    Inner: CompositeFmt<T>,
    Absent: AbsentFmt,
{
    type Decoded<'de> = Inner::Decoded<'de>;

    const WIRE_LEN: usize = N;

    #[inline(always)]
    fn encode_present(output: &mut [u8], scratch: &mut &mut [u8], value: &T) -> Result<(), StructError> {
        let mut slot_out = output;
        Inner::encode_cursor(&mut slot_out, scratch, value)?;
        if !slot_out.is_empty() {
            crate::utils::cold_path();
            return Err(Error::Internal.into());
        }
        Ok(())
    }

    #[inline(always)]
    fn decode_present<'de>(input: &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self::Decoded<'de>, StructError> {
        let mut slot_in = input;
        let value = Inner::decode_cursor(&mut slot_in, scratch)?;
        if !slot_in.is_empty() {
            crate::utils::cold_path();
            return Err(Error::Invalid.into());
        }
        Ok(value)
    }

    #[inline(always)]
    fn encode_absent_slots(output: &mut [u8], scratch: &mut &mut [u8]) -> Result<(), StructError> {
        if output.is_empty() {
            return Ok(());
        }
        let absent = take_scratch(scratch, Self::WIRE_LEN)?;
        let mut slot_out = &mut absent[..];
        Absent::encode_absent(&mut slot_out, scratch)?;
        if !slot_out.is_empty() {
            crate::utils::cold_path();
            return Err(Error::Internal.into());
        }
        fill_repeated_block(output, 0, absent)?;
        Ok(())
    }

    #[inline(always)]
    fn validate_absent_slots(input: &[u8], scratch: &mut &mut [u8]) -> Result<(), StructError> {
        if input.is_empty() {
            return Ok(());
        }
        let absent = take_scratch(scratch, Self::WIRE_LEN)?;
        let mut slot_out = &mut absent[..];
        Absent::encode_absent(&mut slot_out, scratch)?;
        if !slot_out.is_empty() {
            crate::utils::cold_path();
            return Err(Error::Internal.into());
        }
        validate_repeating_block(input, absent)?;
        Ok(())
    }
}

#[inline(always)]
fn fixed_area_lens(slot_len: usize, max: usize) -> Result<usize, Error> {
    if slot_len == 0 {
        crate::utils::cold_path();
        return Err(Error::Internal);
    }
    slot_len.checked_mul(max).ok_or_else(|| {
        crate::utils::cold_path();
        Error::BufferOverflow
    })
}

impl<T, Len, Slot, const MAX: usize> CompositeFmt<Vec<T>> for FixedAreaList<T, Len, Slot, MAX>
where
    Len: LengthSpec<Identity>,
    Slot: FixedAreaSlot<T>,
{
    type Decoded<'de> = Vec<Slot::Decoded<'de>>;

    #[inline(always)]
    fn encode_cursor(output: &mut &mut [u8], scratch: &mut &mut [u8], value: &Vec<T>) -> Result<(), StructError> {
        if value.len() > MAX {
            crate::utils::cold_path();
            return Err(Error::Invalid.into());
        }

        let area_len = fixed_area_lens(Slot::WIRE_LEN, MAX)?;
        let logical_len = value.len().checked_mul(Slot::WIRE_LEN).ok_or_else(|| {
            crate::utils::cold_path();
            StructError::from(Error::BufferOverflow)
        })?;
        Len::encode(output, scratch, logical_len, logical_len)?;

        let area = output.split_off_mut(..area_len).ok_or_else(|| {
            crate::utils::cold_path();
            StructError::from(Error::BufferOverflow)
        })?;
        let mut area_out = area;
        for item in value {
            let slot = area_out.split_off_mut(..Slot::WIRE_LEN).ok_or_else(|| {
                crate::utils::cold_path();
                StructError::from(Error::Internal)
            })?;
            Slot::encode_present(slot, scratch, item)?;
        }
        Slot::encode_absent_slots(area_out, scratch)?;
        Ok(())
    }

    #[inline(always)]
    fn decode_cursor<'de>(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self::Decoded<'de>, StructError> {
        let plan = Len::decode_plan(input, scratch)?;
        let logical_len = plan.exact_len.unwrap_or(plan.output_cap);
        let area_len = fixed_area_lens(Slot::WIRE_LEN, MAX)?;
        if logical_len > area_len || !logical_len.is_multiple_of(Slot::WIRE_LEN) {
            crate::utils::cold_path();
            return Err(Error::Invalid.into());
        }

        let area = input.split_off(..area_len).ok_or_else(|| {
            crate::utils::cold_path();
            StructError::from(Error::UnexpectedEof)
        })?;
        let count = logical_len / Slot::WIRE_LEN;
        let mut values = Vec::with_capacity(count);
        let mut slots = area;
        for _ in 0..count {
            let slot = slots.split_off(..Slot::WIRE_LEN).ok_or_else(|| {
                crate::utils::cold_path();
                StructError::from(Error::Internal)
            })?;
            values.push(Slot::decode_present(slot, scratch)?);
        }
        Slot::validate_absent_slots(slots, scratch)?;
        Ok(values)
    }
}
