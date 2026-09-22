use super::*;
use crate::field::{Identity, LengthSpec};
use crate::primitive::bytes::{contains_byte, fill_repeated_block, split_delimited_bytes, validate_repeated_block};
use crate::utils::take_scratch;

impl ListCountPolicy for () {
    const HAS_COUNT: bool = false;

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
    const HAS_COUNT: bool = true;

    #[inline(always)]
    fn encode_count(output: &mut &mut [u8], scratch: &mut &mut [u8], len: usize) -> Result<(), Error> {
        F::encode(output, scratch, len, len)
    }

    #[inline(always)]
    fn decode_count<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<Option<usize>, Error> {
        let plan = F::decode_plan(input, scratch)?;
        Ok(Some(plan.semantic_len.unwrap_or(plan.wire_len)))
    }
}

impl<const COUNT: usize> ListCountPolicy for FixedCount<COUNT> {
    const HAS_COUNT: bool = true;

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
    fn encode_cursor(output: &mut &mut [u8], scratch: &mut &mut [u8], value: &Vec<T>) -> Result<(), CompositeError> {
        if value.len() > MAX {
            crate::utils::cold_path();
            return Err(Error::Invalid.into());
        }

        Count::encode_count(output, scratch, value.len())?;
        for (index, item) in value.iter().enumerate() {
            if index != 0 {
                encode_list_separator::<Sep>(output)?;
            }
            let available = output.len();
            let used = {
                let mut item_out = &mut **output;
                Item::encode_cursor(&mut item_out, scratch, item)?;
                available - item_out.len()
            };
            let encoded = output.split_off_mut(..used).ok_or_else(|| {
                crate::utils::cold_path();
                CompositeError::from(Error::Internal)
            })?;
            if Sep::BYTE.is_some_and(|separator| (!Count::HAS_COUNT || index + 1 != value.len()) && contains_byte(encoded, separator))
                || (!Count::HAS_COUNT && used == 0 && (Sep::BYTE.is_none() || value.len() == 1))
            {
                crate::utils::cold_path();
                return Err(Error::Invalid.into());
            }
        }
        Ok(())
    }

    #[inline(always)]
    fn decode_cursor<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<Self::Decoded<'a>, CompositeError> {
        let expected = Count::decode_count(input, scratch)?;
        debug_assert_eq!(expected.is_some(), Count::HAS_COUNT);
        if expected.is_some_and(|count| count > MAX) {
            crate::utils::cold_path();
            return Err(Error::Invalid.into());
        }
        let mut values = Vec::with_capacity(expected.unwrap_or(0));

        match (expected, Sep::BYTE) {
            (Some(count), Some(separator)) => {
                for index in 0..count {
                    let mut segment = decode_delimited_field(input, separator, index + 1 != count)?;
                    let value = Item::decode_cursor(&mut segment, scratch)?;
                    if !segment.is_empty() {
                        crate::utils::cold_path();
                        return Err(Error::Invalid.into());
                    }
                    values.push(value);
                }
            }
            (None, Some(separator)) => {
                let mut more = !input.is_empty();
                while more {
                    if values.len() == MAX {
                        crate::utils::cold_path();
                        return Err(Error::Invalid.into());
                    }
                    let (mut segment, terminated) = split_delimited_bytes(input, separator);
                    more = terminated;
                    let value = Item::decode_cursor(&mut segment, scratch)?;
                    if !segment.is_empty() {
                        crate::utils::cold_path();
                        return Err(Error::Invalid.into());
                    }
                    values.push(value);
                }
            }
            (Some(count), None) => {
                for _ in 0..count {
                    values.push(Item::decode_cursor(input, scratch)?);
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

    fn encode_present(output: &mut [u8], scratch: &mut &mut [u8], value: &T) -> Result<(), CompositeError>;
    fn decode_present<'de>(input: &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self::Decoded<'de>, CompositeError>;
    fn encode_absent_slots(output: &mut [u8], scratch: &mut &mut [u8]) -> Result<(), CompositeError>;
    fn validate_absent_slots(input: &[u8], scratch: &mut &mut [u8]) -> Result<(), CompositeError>;
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
    fn encode_present(output: &mut [u8], scratch: &mut &mut [u8], value: &T) -> Result<(), CompositeError> {
        let mut slot_out = output;
        Inner::encode_cursor(&mut slot_out, scratch, value)?;
        if !slot_out.is_empty() {
            crate::utils::cold_path();
            return Err(Error::Internal.into());
        }
        Ok(())
    }

    #[inline(always)]
    fn decode_present<'de>(input: &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self::Decoded<'de>, CompositeError> {
        let mut slot_in = input;
        let value = Inner::decode_cursor(&mut slot_in, scratch)?;
        if !slot_in.is_empty() {
            crate::utils::cold_path();
            return Err(Error::Invalid.into());
        }
        Ok(value)
    }

    #[inline(always)]
    fn encode_absent_slots(output: &mut [u8], scratch: &mut &mut [u8]) -> Result<(), CompositeError> {
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
    fn validate_absent_slots(input: &[u8], scratch: &mut &mut [u8]) -> Result<(), CompositeError> {
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
        validate_repeated_block(input, absent)?;
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
    fn encode_cursor(output: &mut &mut [u8], scratch: &mut &mut [u8], value: &Vec<T>) -> Result<(), CompositeError> {
        if value.len() > MAX {
            crate::utils::cold_path();
            return Err(Error::Invalid.into());
        }

        let area_len = fixed_area_lens(Slot::WIRE_LEN, MAX)?;
        let logical_len = value.len().checked_mul(Slot::WIRE_LEN).ok_or_else(|| {
            crate::utils::cold_path();
            CompositeError::from(Error::BufferOverflow)
        })?;
        Len::encode(output, scratch, logical_len, logical_len)?;

        let area = output.split_off_mut(..area_len).ok_or_else(|| {
            crate::utils::cold_path();
            CompositeError::from(Error::BufferOverflow)
        })?;
        let mut area_out = area;
        for item in value {
            let slot = area_out.split_off_mut(..Slot::WIRE_LEN).ok_or_else(|| {
                crate::utils::cold_path();
                CompositeError::from(Error::Internal)
            })?;
            Slot::encode_present(slot, scratch, item)?;
        }
        Slot::encode_absent_slots(area_out, scratch)?;
        Ok(())
    }

    #[inline(always)]
    fn decode_cursor<'de>(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self::Decoded<'de>, CompositeError> {
        let plan = Len::decode_plan(input, scratch)?;
        let logical_len = plan.semantic_len.unwrap_or(plan.wire_len);
        let area_len = fixed_area_lens(Slot::WIRE_LEN, MAX)?;
        if logical_len > area_len || !logical_len.is_multiple_of(Slot::WIRE_LEN) {
            crate::utils::cold_path();
            return Err(Error::Invalid.into());
        }

        let area = input.split_off(..area_len).ok_or_else(|| {
            crate::utils::cold_path();
            CompositeError::from(Error::UnexpectedEof)
        })?;
        let count = logical_len / Slot::WIRE_LEN;
        let mut values = Vec::with_capacity(count);
        let mut slots = area;
        for _ in 0..count {
            let slot = slots.split_off(..Slot::WIRE_LEN).ok_or_else(|| {
                crate::utils::cold_path();
                CompositeError::from(Error::Internal)
            })?;
            values.push(Slot::decode_present(slot, scratch)?);
        }
        Slot::validate_absent_slots(slots, scratch)?;
        Ok(values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Ascii, AsciiLength, DirectScalar, Ebcdic037, Field, Fixed, Rest};

    type Text = DirectScalar<Field<Ascii<0, 64>, Rest>>;
    type One = DirectScalar<Field<Ascii<1, 1>, Fixed<1>>>;
    pub(super) type Delimited = BoundedList<String, (), Text, Separator<b'|'>, 4>;
    pub(super) type Counted = BoundedList<String, AsciiLength<1>, Text, Separator<b'|'>, 4>;

    pub(super) fn encode<F: CompositeFmt<Vec<String>>>(texts: &[&str]) -> Result<Vec<u8>, Error> {
        let values = texts.iter().map(|s| (*s).to_owned()).collect();
        let mut output = [0; 128];
        let mut out = output.as_mut_slice();
        F::encode(&mut out, &mut [0; 128], &values).map_err(|e| e.kind)?;
        let used = 128 - out.len();
        Ok(output[..used].to_vec())
    }

    fn decode<F>(wire: &[u8]) -> Result<(Vec<String>, Vec<u8>), Error>
    where
        F: for<'de> CompositeFmt<Vec<String>, Decoded<'de> = Vec<String>>,
    {
        let mut input = wire;
        let mut scratch = [0; 128];
        let values = F::decode(&mut input, &mut scratch).map_err(|e| e.kind)?;
        Ok((values, input.to_vec()))
    }

    pub(super) fn roundtrip<F>(texts: &[&str], wire: &[u8])
    where
        F: for<'de> CompositeFmt<Vec<String>, Decoded<'de> = Vec<String>>,
    {
        assert_eq!(encode::<F>(texts).unwrap(), wire);
        let (values, rest) = decode::<F>(wire).unwrap();
        assert_eq!(values, texts);
        assert!(rest.is_empty());
    }

    #[test]
    fn delimited_lists_preserve_empty_items() {
        for (texts, wire) in [
            (&[][..], ""),
            (&["A"][..], "A"),
            (&["", "A"][..], "|A"),
            (&["A", ""][..], "A|"),
            (&["", ""][..], "|"),
            (&["", "", ""][..], "||"),
            (&["A", "", "B", ""][..], "A||B|"),
        ] {
            roundtrip::<Delimited>(texts, wire.as_bytes());
            roundtrip::<Counted>(texts, format!("{}{wire}", texts.len()).as_bytes());
        }
        roundtrip::<Counted>(&[""], b"1");
        assert_eq!(encode::<Delimited>(&[""]), Err(Error::Invalid));
        assert_eq!(decode::<Delimited>(b"A|B|C|D|"), Err(Error::Invalid));
    }

    #[test]
    fn only_final_counted_items_may_contain_wire_separators() {
        for texts in [&["A|B"][..], &["A|B", "C"], &["A", "B|C"]] {
            assert_eq!(encode::<Delimited>(texts), Err(Error::Invalid));
        }
        assert_eq!(encode::<Counted>(&["A|B", "C"]), Err(Error::Invalid));
        assert_eq!(encode::<Counted>(&["A", "B|C", "D"]), Err(Error::Invalid));
        for (texts, wire) in [(&["A|B"][..], &b"1A|B"[..]), (&["A", "B|C"], b"2A|B|C"), (&["A", "|"], b"2A||")] {
            roundtrip::<Counted>(texts, wire);
        }
        for wire in [b"2AB".as_slice(), b"3A|B"] {
            assert_eq!(decode::<Counted>(wire), Err(Error::Invalid));
        }
        type Encoded = BoundedList<String, AsciiLength<1>, DirectScalar<Field<Ascii<1, 1>, Fixed<1>, Ebcdic037>>, Separator<0xC1>, 4>;
        assert_eq!(encode::<Encoded>(&["A", "B"]), Err(Error::Invalid));
        roundtrip::<Encoded>(&["B", "A"], b"2\xC2\xC1\xC1");
        let mut output = [0; 4];
        Counted::encode(&mut output.as_mut_slice(), &mut [], &vec!["A".into(), "B".into()]).unwrap();
        assert_eq!(output, *b"2A|B");
    }

    #[test]
    fn zero_width_items_require_count_or_separators() {
        type Plain = BoundedList<String, (), Text, (), 4>;
        for texts in [&[""][..], &["A", ""], &["", "A"]] {
            assert_eq!(encode::<Plain>(texts), Err(Error::Invalid));
        }
        type Zero = FixedCountList<(), Empty<()>, 2>;
        let mut input = &b"TAIL"[..];
        Zero::encode(&mut &mut [][..], &mut [], &vec![(), ()]).unwrap();
        assert_eq!(Zero::decode(&mut input, &mut []).unwrap(), [(), ()]);
        assert_eq!(input, b"TAIL");
        type CountedZero = BoundedList<(), AsciiLength<1>, Empty<()>, (), 4>;
        let mut output = [0; 1];
        CountedZero::encode(&mut output.as_mut_slice(), &mut [], &vec![(), ()]).unwrap();
        assert_eq!(output, *b"2");
        let mut input = &b"2TAIL"[..];
        assert_eq!(CountedZero::decode(&mut input, &mut []).unwrap(), [(), ()]);
        assert_eq!(input, b"TAIL");
        type UncountedZero = BoundedList<(), (), Empty<()>, (), 4>;
        assert_eq!(UncountedZero::decode(&mut input, &mut []).unwrap_err().kind, Error::Internal);

        type PlainOne = BoundedList<String, (), One, (), 4>;
        type CountedOne = BoundedList<String, AsciiLength<1>, One, (), 4>;
        roundtrip::<PlainOne>(&["A", "|"], b"A|");
        roundtrip::<CountedOne>(&["A", "|"], b"2A|");
        assert_eq!(decode::<CountedOne>(b"2ABTAIL").unwrap().1, b"TAIL");
        assert_eq!(decode::<CountedOne>(b"0TAIL").unwrap().1, b"TAIL");
        assert_eq!(decode::<Counted>(b"0TAIL").unwrap().1, b"TAIL");
    }

    #[test]
    fn list_items_can_borrow_disjoint_scratch() {
        type Borrowed = DirectScalar<Field<Ascii<1, 1>, Fixed<1>, Ebcdic037>, &'static str>;
        type List = BoundedList<&'static str, AsciiLength<1>, Borrowed, Separator<b'|'>, 4>;
        let mut input = &b"2\xC1|\xC2"[..];
        let mut scratch = [0; 2];
        let start = scratch.as_ptr();
        let mut workspace = scratch.as_mut_slice();
        let values = List::decode_cursor(&mut input, &mut workspace).unwrap();
        assert_eq!(values, ["A", "B"]);
        assert_eq!(values[0].as_ptr(), start);
        assert_eq!(values[1].as_ptr(), start.wrapping_add(1));
        assert!(input.is_empty());
        assert!(workspace.is_empty());
    }

    #[test]
    fn count_limits_are_checked_before_reservation() {
        type Huge = BoundedList<String, crate::Length<crate::FixedBinaryBe<8>>, One, (), { usize::MAX - 1 }>;
        let count = (usize::MAX as u64).to_be_bytes();
        assert_eq!(decode::<Huge>(&count), Err(Error::Invalid));

        type Small = BoundedList<String, crate::Length<crate::FixedBinaryBe<8>>, One, (), 3>;
        assert_eq!(decode::<Small>(&count), Err(Error::Invalid));
        type Plain = BoundedList<String, AsciiLength<1>, One, (), 4>;
        type NoItems = BoundedList<String, AsciiLength<1>, One, (), 0>;
        assert_eq!(decode::<Counted>(b"5"), Err(Error::Invalid));
        assert_eq!(decode::<Plain>(b"5"), Err(Error::Invalid));
        assert_eq!(decode::<NoItems>(b"1"), Err(Error::Invalid));
        assert_eq!(decode::<NoItems>(b"0TAIL"), Ok((vec![], b"TAIL".to_vec())));
        roundtrip::<Plain>(&["A", "B", "C", "D"], b"4ABCD");
        assert_eq!(decode::<Plain>(b"4A"), Err(Error::UnexpectedEof));
    }
}

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    use super::tests::{Counted, Delimited, encode, roundtrip};
    use crate::Error;

    proptest! {
        #[test]
        fn delimited_list_grammar_roundtrips(values in prop::collection::vec("[A-Z|]{0,8}", 0..=4)) {
            let texts: Vec<_> = values.iter().map(String::as_str).collect();
            let body = texts.join("|");
            if texts.iter().take(texts.len().saturating_sub(1)).all(|s| !s.contains('|')) {
                roundtrip::<Counted>(&texts, format!("{}{body}", texts.len()).as_bytes());
            } else {
                prop_assert_eq!(encode::<Counted>(&texts), Err(Error::Invalid));
            }
            if texts.iter().all(|s| !s.contains('|')) && !(texts.len() == 1 && texts[0].is_empty()) {
                roundtrip::<Delimited>(&texts, body.as_bytes());
            } else {
                prop_assert_eq!(encode::<Delimited>(&texts), Err(Error::Invalid));
            }
        }
    }
}
