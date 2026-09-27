use super::*;
use crate::field::LengthSpec;
use crate::primitive::bytes::{take_bytes, take_delimited};

impl ListSeparatorPolicy for () {
    const BYTE: Option<u8> = None;
}

impl<const BYTE: u8> ListSeparatorPolicy for Separator<BYTE> {
    const BYTE: Option<u8> = Some(BYTE);
}

/// `MAX` bounds the list's length, so a prefix that cannot hold it means the
/// list is written wrong. Debug builds catch it; release passes the error through.
#[inline(always)]
fn framed(result: Result<(), Error>) -> Result<(), Error> {
    debug_assert!(
        !matches!(result, Err(Error::Invalid)),
        "the list's MAX exceeds what its length prefix can hold"
    );
    result
}

#[inline]
fn encode_list_separator<S: ListSeparatorPolicy>(output: &mut &mut [u8]) -> Result<(), Error> {
    if let Some(byte) = S::BYTE {
        encode_delimiter(output, byte)?;
    }
    Ok(())
}

/// Encode the items with their separators. `counted` says whether decoding
/// knows the item count, which decides what keeps the wire unambiguous.
#[inline(always)]
fn encode_items<T, Item: CompositeFmt<T>, Sep: ListSeparatorPolicy>(
    output: &mut &mut [u8],
    scratch: &mut [u8],
    value: &[T],
    counted: bool,
) -> Result<(), CompositeError> {
    for (index, item) in value.iter().enumerate() {
        if index != 0 {
            encode_list_separator::<Sep>(output)?;
        }
        // The item's format must keep the wire unambiguous; debug builds check it.
        let encode = |out: &mut &mut [u8], scratch: &mut [u8]| Item::encode(out, scratch, item).map_err(|error| error.with_index(index));
        encode_debug_checked(output, scratch, encode, |encoded| {
            debug_assert!(
                Sep::BYTE.is_none_or(|separator| counted && index + 1 == value.len() || !encoded.contains(&separator)),
                "a list item encoded the separator; its check must exclude it"
            );
            debug_assert!(
                counted || !encoded.is_empty() || Sep::BYTE.is_some() && value.len() != 1,
                "an uncounted list item encoded to nothing, which decodes differently"
            );
        })?;
    }
    Ok(())
}

impl<T, L, Item, Sep, const MIN: usize, const MAX: usize> CompositeFmt<Vec<T>> for BoundedList<T, L, Item, Sep, MIN, MAX>
where
    L: LengthSpec,
    Item: CompositeFmt<T>,
    Sep: ListSeparatorPolicy,
{
    type Decoded<'de> = Vec<Item::Decoded<'de>>;

    #[inline(always)]
    fn encode(output: &mut &mut [u8], scratch: &mut [u8], value: &Vec<T>) -> Result<(), CompositeError> {
        const { assert!(MIN <= MAX, "a list's MIN must not exceed its MAX") };
        if value.len() < MIN || value.len() > MAX {
            crate::utils::cold_path();
            return Err(Error::InvalidValueLength.into());
        }
        framed(L::encode(output, scratch, value.len()))?;
        encode_items::<T, Item, Sep>(output, scratch, value, L::STATES_LEN)
    }

    #[inline(always)]
    fn decode<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<Self::Decoded<'a>, CompositeError> {
        let values = match L::decode(input, scratch)? {
            Some(count) => {
                if count < MIN || count > MAX {
                    crate::utils::cold_path();
                    return Err(Error::Invalid.into());
                }
                // A declared count is untrusted input: reserve no more items than bytes remain.
                let mut values = Vec::with_capacity(count.min(input.len()));
                if let Some(separator) = Sep::BYTE {
                    for index in 0..count {
                        let mut segment = decode_delimited_field(input, separator, index + 1 != count)?;
                        let value = Item::decode(&mut segment, scratch).map_err(|error| error.with_index(index))?;
                        if !segment.is_empty() {
                            crate::utils::cold_path();
                            return Err(Error::Invalid.into());
                        }
                        values.push(value);
                    }
                } else {
                    for index in 0..count {
                        values.push(Item::decode(input, scratch).map_err(|error| error.with_index(index))?);
                    }
                }
                values
            }
            None => {
                let values = decode_uncounted::<T, Item, Sep, MAX>(input, scratch)?;
                if values.len() < MIN {
                    crate::utils::cold_path();
                    return Err(Error::Invalid.into());
                }
                values
            }
        };
        Ok(values)
    }
}

/// Decode items until `input` is used up, at most `MAX` of them.
#[inline(always)]
fn decode_uncounted<'a, T, Item: CompositeFmt<T>, Sep: ListSeparatorPolicy, const MAX: usize>(
    input: &mut &'a [u8],
    scratch: &mut &'a mut [u8],
) -> Result<Vec<Item::Decoded<'a>>, CompositeError> {
    let mut values = Vec::new();
    if let Some(separator) = Sep::BYTE {
        let mut more = !input.is_empty();
        while more {
            if values.len() == MAX {
                crate::utils::cold_path();
                return Err(Error::Invalid.into());
            }
            let (mut segment, terminated) = take_delimited(input, separator);
            more = terminated;
            let value = Item::decode(&mut segment, scratch).map_err(|error| error.with_index(values.len()))?;
            if !segment.is_empty() {
                crate::utils::cold_path();
                return Err(Error::Invalid.into());
            }
            values.push(value);
        }
    } else {
        while !input.is_empty() {
            if values.len() == MAX {
                crate::utils::cold_path();
                return Err(Error::Invalid.into());
            }
            let before = input.len();
            let value = Item::decode(input, scratch).map_err(|error| error.with_index(values.len()))?;
            // An item that consumes nothing would loop forever.
            if input.len() == before {
                crate::utils::cold_path();
                return Err(Error::Internal.into());
            }
            values.push(value);
        }
    }
    Ok(values)
}

mod sealed {
    pub trait FixedAreaSlotSealed {}
}

#[doc(hidden)]
pub trait FixedAreaSlot<T>: sealed::FixedAreaSlotSealed {
    type Decoded<'de>;

    const WIRE_LEN: usize;

    fn encode_present(output: &mut &mut [u8], scratch: &mut [u8], value: &T) -> Result<(), CompositeError>;
    fn decode_present<'de>(input: &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self::Decoded<'de>, CompositeError>;
    fn encode_absent_slots(output: &mut &mut [u8], scratch: &mut [u8], count: usize) -> Result<(), CompositeError>;
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
    fn encode_present(output: &mut &mut [u8], scratch: &mut [u8], value: &T) -> Result<(), CompositeError> {
        const { assert!(N != 0, "an OptionalAbsent area must be at least one byte wide") };
        let available = output.len();
        Inner::encode(output, scratch, value)?;
        debug_assert_eq!(available - output.len(), N, "the value is not as wide as its slot");
        Ok(())
    }

    #[inline(always)]
    fn decode_present<'de>(input: &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self::Decoded<'de>, CompositeError> {
        let mut slot_in = input;
        let value = Inner::decode(&mut slot_in, scratch)?;
        if !slot_in.is_empty() {
            crate::utils::cold_path();
            return Err(Error::Invalid.into());
        }
        Ok(value)
    }

    #[inline(always)]
    fn encode_absent_slots(output: &mut &mut [u8], scratch: &mut [u8], count: usize) -> Result<(), CompositeError> {
        for _ in 0..count {
            Absent::encode_absent(output, scratch, N)?;
        }
        Ok(())
    }

    #[inline(always)]
    fn validate_absent_slots(input: &[u8], scratch: &mut &mut [u8]) -> Result<(), CompositeError> {
        if input.is_empty() {
            return Ok(());
        }
        for slot in input.as_chunks::<N>().0 {
            if !Absent::is_absent(slot, &mut &mut **scratch)? {
                crate::utils::cold_path();
                return Err(Error::Invalid.into());
            }
        }
        Ok(())
    }
}

#[inline(always)]
fn fixed_area_lens(slot_len: usize, max: usize) -> Result<usize, Error> {
    slot_len.checked_mul(max).ok_or_else(|| {
        crate::utils::cold_path();
        Error::BufferOverflow
    })
}

impl<T, Len, Slot, const MAX: usize> CompositeFmt<Vec<T>> for FixedAreaList<T, Len, Slot, MAX>
where
    Len: LengthSpec,
    Slot: FixedAreaSlot<T>,
{
    type Decoded<'de> = Vec<Slot::Decoded<'de>>;

    #[inline(always)]
    fn encode(output: &mut &mut [u8], scratch: &mut [u8], value: &Vec<T>) -> Result<(), CompositeError> {
        if value.len() > MAX {
            crate::utils::cold_path();
            return Err(Error::InvalidValueLength.into());
        }

        const { assert!(Len::STATES_LEN, "a fixed area needs a declared used extent") };
        framed(Len::encode(output, scratch, value.len()))?;

        // Write each slot in turn, so nothing is reserved and filled later.
        for (index, item) in value.iter().enumerate() {
            Slot::encode_present(output, scratch, item).map_err(|error| error.with_index(index))?;
        }
        Slot::encode_absent_slots(output, scratch, MAX - value.len())?;
        Ok(())
    }

    #[inline(always)]
    fn decode<'de>(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self::Decoded<'de>, CompositeError> {
        const { assert!(Slot::WIRE_LEN != 0, "fixed-area slots must be at least one byte wide") };
        const { assert!(Len::STATES_LEN, "a fixed area needs a declared used extent") };
        let count = Len::decode(input, scratch)?.unwrap_or_default();
        if count > MAX {
            crate::utils::cold_path();
            return Err(Error::Invalid.into());
        }
        let area_len = fixed_area_lens(Slot::WIRE_LEN, MAX)?;
        let area = take_bytes(input, area_len)?;
        let mut values = Vec::with_capacity(count);
        let mut slots = area;
        for index in 0..count {
            let slot = slots.split_off(..Slot::WIRE_LEN).ok_or_else(|| {
                crate::utils::cold_path();
                CompositeError::from(Error::Internal)
            })?;
            values.push(Slot::decode_present(slot, scratch).map_err(|error| error.with_index(index))?);
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
    pub(super) type Delimited = BoundedList<String, Rest, Text, Separator<b'|'>, 0, 4>;
    pub(super) type Counted = BoundedList<String, AsciiLength<1>, Text, Separator<b'|'>, 0, 4>;

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
        let values = F::decode(&mut input, &mut &mut scratch[..]).map_err(|e| e.kind)?;
        Ok((values, input.to_vec()))
    }

    /// Encoding an ambiguous list is a composition mistake: debug builds panic,
    /// release builds encode whatever the items produce.
    pub(super) fn rejected_in_debug<F: CompositeFmt<Vec<String>>>(texts: &[&str]) -> bool {
        std::panic::catch_unwind(|| encode::<F>(texts)).is_err() == cfg!(debug_assertions)
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
    fn item_errors_carry_their_index() {
        let error = |result: Result<Vec<String>, CompositeError>| result.unwrap_err().to_string();
        let mut input = &b"3A||B"[..];
        type Counted2 = BoundedList<String, AsciiLength<1>, One, Separator<b'|'>, 0, 4>;
        assert_eq!(
            error(Counted2::decode(&mut input, &mut &mut [0; 8][..])),
            "[1]: unexpected end of input"
        );
        let values = vec!["A".to_owned(), "BC".to_owned()];
        assert_eq!(
            Counted2::encode(&mut [0; 8].as_mut_slice(), &mut [], &values)
                .unwrap_err()
                .to_string(),
            "[1]: value length not accepted by the field"
        );
        type Two = DirectScalar<Field<Ascii<2, 2>, Fixed<2>>>;
        type Area = FixedAreaList<String, AsciiLength<1>, OptionalAbsent<String, Two, ByteFill, 2>, 3>;
        assert_eq!(
            error(Area::decode(&mut &b"2AB\xff\xff  "[..], &mut &mut [][..])),
            "[1]: invalid data"
        );
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
        assert!(rejected_in_debug::<Delimited>(&[""]));
        assert_eq!(decode::<Delimited>(b"A|B|C|D|"), Err(Error::Invalid));
    }

    #[test]
    fn only_final_counted_items_may_contain_wire_separators() {
        for texts in [&["A|B"][..], &["A|B", "C"], &["A", "B|C"]] {
            assert!(rejected_in_debug::<Delimited>(texts));
        }
        assert!(rejected_in_debug::<Counted>(&["A|B", "C"]));
        assert!(rejected_in_debug::<Counted>(&["A", "B|C", "D"]));
        for (texts, wire) in [(&["A|B"][..], &b"1A|B"[..]), (&["A", "B|C"], b"2A|B|C"), (&["A", "|"], b"2A||")] {
            roundtrip::<Counted>(texts, wire);
        }
        for wire in [b"2AB".as_slice(), b"3A|B"] {
            assert_eq!(decode::<Counted>(wire), Err(Error::Invalid));
        }
        type Encoded = BoundedList<String, AsciiLength<1>, DirectScalar<Field<Ascii<1, 1>, Fixed<1>, Ebcdic037>>, Separator<0xC1>, 0, 4>;
        assert!(rejected_in_debug::<Encoded>(&["A", "B"]));
        roundtrip::<Encoded>(&["B", "A"], b"2\xC2\xC1\xC1");
        let mut output = [0; 4];
        Counted::encode(&mut output.as_mut_slice(), &mut [], &vec!["A".into(), "B".into()]).unwrap();
        assert_eq!(output, *b"2A|B");
    }

    #[test]
    fn zero_width_items_require_count_or_separators() {
        type Plain = BoundedList<String, Rest, Text, (), 0, 4>;
        for texts in [&[""][..], &["A", ""], &["", "A"]] {
            assert!(rejected_in_debug::<Plain>(texts));
        }
        type Zero = BoundedList<(), Fixed<2>, Empty<()>, (), 2, 2>;
        let mut input = &b"TAIL"[..];
        Zero::encode(&mut &mut [][..], &mut [], &vec![(), ()]).unwrap();
        assert_eq!(Zero::decode(&mut input, &mut &mut [][..]).unwrap(), [(), ()]);
        assert_eq!(input, b"TAIL");
        type CountedZero = BoundedList<(), AsciiLength<1>, Empty<()>, (), 0, 4>;
        let mut output = [0; 1];
        CountedZero::encode(&mut output.as_mut_slice(), &mut [], &vec![(), ()]).unwrap();
        assert_eq!(output, *b"2");
        let mut input = &b"2TAIL"[..];
        assert_eq!(CountedZero::decode(&mut input, &mut &mut [][..]).unwrap(), [(), ()]);
        assert_eq!(input, b"TAIL");
        type UncountedZero = BoundedList<(), Rest, Empty<()>, (), 0, 4>;
        assert_eq!(
            UncountedZero::decode(&mut input, &mut &mut [][..]).unwrap_err().kind,
            Error::Internal
        );

        type PlainOne = BoundedList<String, Rest, One, (), 0, 4>;
        type CountedOne = BoundedList<String, AsciiLength<1>, One, (), 0, 4>;
        roundtrip::<PlainOne>(&["A", "|"], b"A|");
        roundtrip::<CountedOne>(&["A", "|"], b"2A|");
        assert_eq!(decode::<CountedOne>(b"2ABTAIL").unwrap().1, b"TAIL");
        assert_eq!(decode::<CountedOne>(b"0TAIL").unwrap().1, b"TAIL");
        assert_eq!(decode::<Counted>(b"0TAIL").unwrap().1, b"TAIL");
    }

    #[test]
    fn lists_count_items_and_frames_give_byte_extents() {
        // A byte length is a frame around a list that takes the rest.
        type Extent = Frame<Field<crate::Binary<0, 99>, AsciiLength<2>>, BoundedList<String, Rest, Text, Separator<b'|'>, 0, 4>>;
        roundtrip::<Extent>(&["AB", "C"], b"04AB|C");
        roundtrip::<Extent>(&[], b"00");
        assert_eq!(
            decode::<Extent>(b"04AB|CTAIL").unwrap(),
            (vec!["AB".into(), "C".into()], b"TAIL".to_vec())
        );
        assert_eq!(decode::<Extent>(b"09AB|C"), Err(Error::UnexpectedEof));
        // MIN bounds the list's length like a field check's minimum.
        type AtLeastTwo = BoundedList<String, AsciiLength<1>, One, (), 2, 4>;
        assert_eq!(encode::<AtLeastTwo>(&["A"]), Err(Error::InvalidValueLength));
        assert_eq!(decode::<AtLeastTwo>(b"1A"), Err(Error::Invalid));
        roundtrip::<AtLeastTwo>(&["A", "B"], b"2AB");
        type RestAtLeastOne = BoundedList<String, Rest, One, (), 1, 4>;
        assert_eq!(decode::<RestAtLeastOne>(b""), Err(Error::Invalid));
        assert_eq!(decode::<Extent>(b"05ABCDE").unwrap().0, ["ABCDE"]);
    }

    #[test]
    fn list_items_can_borrow_disjoint_scratch() {
        type Borrowed = DirectScalar<Field<Ascii<1, 1>, Fixed<1>, Ebcdic037>, &'static str>;
        type List = BoundedList<&'static str, AsciiLength<1>, Borrowed, Separator<b'|'>, 0, 4>;
        let mut input = &b"2\xC1|\xC2"[..];
        let mut scratch = [0; 2];
        let start = scratch.as_ptr();
        let mut workspace = scratch.as_mut_slice();
        let values = List::decode(&mut input, &mut workspace).unwrap();
        assert_eq!(values, ["A", "B"]);
        assert_eq!(values[0].as_ptr(), start);
        assert_eq!(values[1].as_ptr(), start.wrapping_add(1));
        assert!(input.is_empty());
        assert!(workspace.is_empty());
    }

    #[test]
    fn count_limits_are_checked_before_reservation() {
        type Huge = BoundedList<String, crate::Length<crate::FixedBinaryBe<8>>, One, (), 0, { usize::MAX - 1 }>;
        let count = (usize::MAX as u64).to_be_bytes();
        assert_eq!(decode::<Huge>(&count), Err(Error::Invalid));
        let within_max = ((usize::MAX - 2) as u64).to_be_bytes();
        assert_eq!(decode::<Huge>(&within_max), Err(Error::UnexpectedEof));

        type Small = BoundedList<String, crate::Length<crate::FixedBinaryBe<8>>, One, (), 0, 3>;
        assert_eq!(decode::<Small>(&count), Err(Error::Invalid));
        type Plain = BoundedList<String, AsciiLength<1>, One, (), 0, 4>;
        type NoItems = BoundedList<String, AsciiLength<1>, One, (), 0, 0>;
        assert_eq!(decode::<Counted>(b"5"), Err(Error::Invalid));
        assert_eq!(decode::<Plain>(b"5"), Err(Error::Invalid));
        assert_eq!(decode::<NoItems>(b"1"), Err(Error::Invalid));
        assert_eq!(decode::<NoItems>(b"0TAIL"), Ok((vec![], b"TAIL".to_vec())));
        roundtrip::<Plain>(&["A", "B", "C", "D"], b"4ABCD");
        assert_eq!(decode::<Plain>(b"4A"), Err(Error::UnexpectedEof));
        // The item count is the list value's own length.
        assert_eq!(encode::<Plain>(&["A"; 5]), Err(Error::InvalidValueLength));
        assert_eq!(
            encode::<BoundedList<String, Fixed<3>, One, (), 3, 3>>(&["A", "B"]),
            Err(Error::InvalidValueLength)
        );
        // A MAX the count prefix cannot hold is a miswritten list.
        type TooWide = BoundedList<String, AsciiLength<1>, One, (), 0, 12>;
        assert!(std::panic::catch_unwind(|| encode::<TooWide>(&["A"; 10])).is_err() == cfg!(debug_assertions));
    }

    struct SpacesOrZeros;

    impl AbsentFmt for SpacesOrZeros {
        fn encode_absent(output: &mut &mut [u8], scratch: &mut [u8], len: usize) -> Result<(), Error> {
            ByteFill::<b' '>::encode_absent(output, scratch, len)
        }

        fn is_absent(input: &[u8], _scratch: &mut &mut [u8]) -> Result<bool, Error> {
            use crate::primitive::bytes::is_filled;
            Ok(is_filled(input, b' ') || is_filled(input, b'0'))
        }
    }

    #[test]
    fn unused_fixed_area_slots_use_the_absent_matcher() {
        type Two = DirectScalar<Field<Ascii<2, 2>, Fixed<2>>>;
        type Optional = OptionalAbsent<String, Two, SpacesOrZeros, 2>;
        type Area = FixedAreaList<String, AsciiLength<1>, Optional, 3>;
        assert_eq!(Optional::decode(&mut &b"00"[..], &mut &mut [][..]), Ok(None));
        // The counting prefix counts used slots.
        for (wire, expected) in [
            (&b"0  00  "[..], &[][..]),
            (b"1AB00  ", &["AB"]),
            (b"1  0000", &["  "]),
            (b"200AB00", &["00", "AB"]),
            (b"3AB00  ", &["AB", "00", "  "]),
        ] {
            let (values, rest) = decode::<Area>(wire).unwrap();
            assert_eq!(values, expected);
            assert!(rest.is_empty());
        }
        assert_eq!(decode::<Area>(b"1AB0000TAIL").unwrap().1, b"TAIL");
        for (wire, error) in [
            (&b"4AB    "[..], Error::Invalid),
            (b"8AB    ", Error::Invalid),
            (b"0AB    ", Error::Invalid),
            (b"0 0    ", Error::Invalid),
            (b"1AB   ", Error::UnexpectedEof),
        ] {
            assert_eq!(decode::<Area>(wire), Err(error));
        }
        roundtrip::<Area>(&[], b"0      ");
        roundtrip::<Area>(&["AB"], b"1AB    ");
        roundtrip::<Area>(&["AB", "00", "  "], b"3AB00  ");
    }

    #[test]
    fn unused_slot_checks_preserve_borrowed_values_and_workspace() {
        struct Canonical;
        impl AbsentFmt for Canonical {
            fn encode_absent(output: &mut &mut [u8], scratch: &mut [u8], len: usize) -> Result<(), Error> {
                ByteFill::<b'_'>::encode_absent(output, scratch, len)
            }
        }
        type Borrowed = DirectScalar<Field<Ascii<1, 1>, Fixed<1>, Ebcdic037>, &'static str>;
        type Area = FixedAreaList<&'static str, AsciiLength<1>, OptionalAbsent<&'static str, Borrowed, Canonical, 1>, 4>;
        let mut input = &b"1\xC1___TAIL"[..];
        let mut scratch = [0; 2];
        let start = scratch.as_ptr();
        let mut workspace = scratch.as_mut_slice();
        let values = Area::decode(&mut input, &mut workspace).unwrap();
        assert_eq!(values, ["A"]);
        assert_eq!(values[0].as_ptr(), start);
        assert_eq!(workspace.len(), 1);
        assert_eq!(input, b"TAIL");
    }
}

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    use super::tests::{Counted, Delimited, rejected_in_debug, roundtrip};

    proptest! {
        #[test]
        fn delimited_list_grammar_roundtrips(values in prop::collection::vec("[A-Z|]{0,8}", 0..=4)) {
            let texts: Vec<_> = values.iter().map(String::as_str).collect();
            let body = texts.join("|");
            if texts.iter().take(texts.len().saturating_sub(1)).all(|s| !s.contains('|')) {
                roundtrip::<Counted>(&texts, format!("{}{body}", texts.len()).as_bytes());
            } else {
                prop_assert!(rejected_in_debug::<Counted>(&texts));
            }
            if texts.iter().all(|s| !s.contains('|')) && !(texts.len() == 1 && texts[0].is_empty()) {
                roundtrip::<Delimited>(&texts, body.as_bytes());
            } else {
                prop_assert!(rejected_in_debug::<Delimited>(&texts));
            }
        }
    }
}
