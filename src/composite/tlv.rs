//! Tagged records: entries of a tag in a fixed-width scalar format followed
//! by a value whose format, declared per tag, carries its own length.

use core::str::FromStr;

use super::*;
use crate::utils::cold_path;

/// The longest tag a TLV record reads, in decoded bytes; a longer one is
/// `Invalid`.
#[doc(hidden)]
pub const MAX_TLV_TAG: usize = 16;

/// Entries of a TLV list, or the unknown entries of a TLV record collected
/// by its `extras` field: the tag's text as the key, and the value read by
/// the value format.
///
/// Implemented for collections of `(key, value)` pairs: a
/// `Vec<(String, String)>` keeps every entry in order, while a
/// `BTreeMap<String, String>` keeps the last value of a repeated tag.
pub trait TlvExtras {
    /// The value of each entry.
    type Value;

    /// Encode the entries, each tag through `T` and value through `F`,
    /// rejecting tags in `known_tags`, a record's declared tags, even if
    /// their fields are absent.
    fn encode_unknowns<T: ScalarFmt, F: FieldEncode<Self::Value>>(
        &self,
        output: &mut &mut [u8],
        scratch: &mut [u8],
        known_tags: &[&str],
    ) -> Result<(), CompositeError>;

    /// Decode the value of an entry whose tag is not declared, through `F`.
    fn decode_unknown<'de, F: FieldDecode<'de, Self::Value>>(
        &mut self,
        tag: &[u8],
        input: &mut &'de [u8],
        scratch: &mut &'de mut [u8],
    ) -> Result<(), CompositeError>;
}

/// An entry borrowed from a collection: `(&K, &V)` from a map, `&(K, V)`
/// from a sequence of pairs.
#[doc(hidden)]
pub trait TlvEntry<'a, K: 'a, V: 'a> {
    fn parts(self) -> (&'a K, &'a V);
}

impl<'a, K, V> TlvEntry<'a, K, V> for (&'a K, &'a V) {
    #[inline(always)]
    fn parts(self) -> (&'a K, &'a V) {
        self
    }
}

impl<'a, K, V> TlvEntry<'a, K, V> for &'a (K, V) {
    #[inline(always)]
    fn parts(self) -> (&'a K, &'a V) {
        (&self.0, &self.1)
    }
}

impl<C, K, V> TlvExtras for C
where
    C: IntoIterator<Item = (K, V)> + Extend<(K, V)>,
    for<'a> &'a C: IntoIterator<Item: TlvEntry<'a, K, V>>,
    K: AsRef<str> + FromStr,
{
    type Value = V;

    #[inline(always)]
    fn encode_unknowns<T: ScalarFmt, F: FieldEncode<V>>(
        &self,
        output: &mut &mut [u8],
        scratch: &mut [u8],
        known_tags: &[&str],
    ) -> Result<(), CompositeError> {
        for entry in self {
            let (key, value) = entry.parts();
            let key = key.as_ref();
            if known_tags.contains(&key) {
                cold_path();
                return Err(Error::Invalid.into());
            }
            T::encode(output, scratch, key.as_bytes())?;
            F::encode_field(output, scratch, value)?;
        }
        Ok(())
    }

    #[inline(always)]
    fn decode_unknown<'de, F: FieldDecode<'de, V>>(
        &mut self,
        tag: &[u8],
        input: &mut &'de [u8],
        scratch: &mut &'de mut [u8],
    ) -> Result<(), CompositeError> {
        let key = core::str::from_utf8(tag)
            .ok()
            .and_then(|tag| tag.parse::<K>().ok())
            .ok_or_else(|| {
                cold_path();
                Error::Invalid
            })?;
        let value = F::decode_field(input, scratch)?;
        self.extend(core::iter::once((key, value)));
        Ok(())
    }
}

/// Bytes a [`TlvList`] skips before, between and after entries.
pub trait TlvPadding {
    /// The padding bytes; empty for none.
    const BYTES: &'static [u8];
}

/// No padding between TLV entries.
pub struct NoPadding;

impl TlvPadding for NoPadding {
    const BYTES: &'static [u8] = &[];
}

/// Padding of `BYTE` before, between and after TLV entries, such as `00` in
/// BER-TLV. Decoding skips it; encoding never writes it.
pub struct PaddingByte<const BYTE: u8>;

impl<const BYTE: u8> TlvPadding for PaddingByte<BYTE> {
    const BYTES: &'static [u8] = &[BYTE];
}

/// A list of tag-length-value entries that all share one value format, into
/// any [`TlvExtras`] collection: what a `tlv` record with only an `extras`
/// field reads, without declared tags. Each tag is written by the scalar
/// format `T`, which must read a tag of a known width, and each value by `F`,
/// which carries its own length; `P` is the [`TlvPadding`].
///
/// BER-TLV with hex values, keeping every entry in order:
///
/// ```
/// use finfmt::primitive::nibble::UpperHexDigits;
/// use finfmt::{BerLength, BerTag, Field, PackNibbles, PaddingByte, TlvList, UpperHexEven};
///
/// type BerHex = Field<UpperHexEven<0, 512>, BerLength, PackNibbles<UpperHexDigits>>;
/// type Emv = TlvList<BerTag, BerHex, PaddingByte<0x00>>;
///
/// let mut scratch = [0; 64];
/// let entries: Vec<(String, String)> = finfmt::decode::<Emv, _>(b"\x9F\x02\x02\x12\x34\x00\x5A\x00", &mut scratch).unwrap();
/// assert_eq!(entries, [("9F02".into(), "1234".into()), ("5A".into(), "".into())]);
/// ```
pub struct TlvList<T, F, P = NoPadding>(PhantomData<(T, F, P)>);

impl<C: TlvExtras, T: ScalarFmt, F: FieldEncode<C::Value>, P> FieldEncode<C> for TlvList<T, F, P> {
    #[inline(always)]
    fn encode_field(output: &mut &mut [u8], scratch: &mut [u8], value: &C) -> Result<(), CompositeError> {
        value.encode_unknowns::<T, F>(output, scratch, &[])
    }
}

impl<'de, C: TlvExtras + Default, T: ScalarFmt, F: FieldDecode<'de, C::Value>, P: TlvPadding> FieldDecode<'de, C> for TlvList<T, F, P> {
    // Entries are read until the input ends.
    const TAKES_REST: bool = true;

    #[inline(always)]
    fn decode_field(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<C, CompositeError> {
        let mut entries = C::default();
        skip_tlv_padding(input, P::BYTES);
        while !input.is_empty() {
            decode_tlv_unknown::<T, F, C>(input, scratch, &mut entries)?;
            skip_tlv_padding(input, P::BYTES);
        }
        Ok(entries)
    }
}

/// Decode an entry no declared tag matched into `entries`: its tag through
/// `T`, kept only as text, so its scratch is not kept, and its value through
/// `F`.
#[doc(hidden)]
#[inline(always)]
pub fn decode_tlv_unknown<'de, T: ScalarFmt, F: FieldDecode<'de, C::Value>, C: TlvExtras>(
    input: &mut &'de [u8],
    scratch: &mut &'de mut [u8],
    entries: &mut C,
) -> Result<(), CompositeError> {
    let mut tag = [0; MAX_TLV_TAG];
    let tag = decode_tlv_tag::<T>(input, scratch, &mut tag)?;
    entries.decode_unknown::<F>(tag, input, scratch)
}

/// Write a declared field's tag through `T`.
#[doc(hidden)]
#[inline(always)]
pub fn encode_tlv_tag<T: ScalarFmt>(
    output: &mut &mut [u8],
    scratch: &mut [u8],
    tag: &str,
    field: &'static str,
) -> Result<(), CompositeError> {
    let result = T::encode(output, scratch, tag.as_bytes());
    debug_assert!(
        !matches!(result, Err(Error::Invalid | Error::InvalidValueLength)),
        "a declared tag is not a valid value of the record's tag format"
    );
    result.map_err(|error| wrap_composite_error(error, field))
}

/// A declared tag's wire bytes, encoded once per decode and matched against
/// each entry.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct TlvTag {
    bytes: [u8; MAX_TLV_TAG],
    len: usize,
}

impl TlvTag {
    /// No tag: the place of an `extras` field, which matches nothing.
    pub const NONE: Self = Self {
        bytes: [0; MAX_TLV_TAG],
        len: 0,
    };

    /// Encode a declared tag's text through `T`. Its scratch is not kept.
    #[inline(always)]
    pub fn encode<T: ScalarFmt>(scratch: &mut [u8], tag: &str) -> Result<Self, Error> {
        let mut out = Self::NONE;
        let mut cursor = out.bytes.as_mut_slice();
        let available = cursor.len();
        let result = T::encode(&mut cursor, scratch, tag.as_bytes());
        debug_assert!(
            result.is_ok(),
            "a declared tag is not a valid value of the record's tag format, or is over 16 bytes"
        );
        result?;
        out.len = available - cursor.len();
        Ok(out)
    }

    /// Whether the tag starts with one of `bytes`, such as padding.
    #[inline(always)]
    pub fn starts_with_any(&self, bytes: &[u8]) -> bool {
        self.bytes
            .get(..self.len)
            .and_then(|tag| tag.first())
            .is_some_and(|first| bytes.contains(first))
    }

    /// Whether `input` starts with this tag; if so, advance past it.
    #[inline(always)]
    pub fn matches(&self, input: &mut &[u8]) -> bool {
        let tag = self.bytes.get(..self.len).unwrap_or_default();
        match input.strip_prefix(tag) {
            Some(rest) if !tag.is_empty() => {
                *input = rest;
                true
            }
            _ => false,
        }
    }
}

/// Skip any padding bytes before the next entry or at the end: each byte of
/// `input` that is in `padding`, which holds the record's padding byte, if
/// any.
#[doc(hidden)]
#[inline(always)]
pub fn skip_tlv_padding(input: &mut &[u8], padding: &[u8]) {
    while let [first, rest @ ..] = *input
        && padding.contains(first)
    {
        *input = rest;
    }
}

/// Read the next entry's tag through `T` into `buf`, for an entry no declared
/// tag matched. The tag is only kept as text, so its scratch is not kept.
#[inline(always)]
fn decode_tlv_tag<'b, T: ScalarFmt>(input: &mut &[u8], scratch: &mut [u8], buf: &'b mut [u8; MAX_TLV_TAG]) -> Result<&'b [u8], Error> {
    let source = *input;
    let mut rest = source;
    let mut workspace = scratch;
    let tag = T::decode(&mut rest, &mut workspace)?;
    let len = tag.len();
    let mut cursor = buf.as_mut_slice();
    copy_bytes(&mut cursor, tag).map_err(|_| {
        cold_path();
        Error::Invalid
    })?;
    advance_input(input, source.len() - rest.len())?;
    buf.get(..len).ok_or_else(|| {
        cold_path();
        Error::Internal
    })
}

/// Decode a declared field's value; a tag seen twice is `Invalid`.
#[doc(hidden)]
#[inline(always)]
pub fn decode_tlv_field<'a, T, D>(
    input: &mut &'a [u8],
    scratch: &mut &'a mut [u8],
    field_value: &mut Option<T>,
    field: &'static str,
    decode_value: D,
) -> Result<(), CompositeError>
where
    D: FnOnce(&mut &'a [u8], &mut &'a mut [u8]) -> Result<T, CompositeError>,
{
    if field_value.is_some() {
        cold_path();
        return Err(wrap_composite_error(Error::Invalid, field));
    }
    *field_value = Some(decode_value(input, scratch).map_err(|error| wrap_composite_error(error, field))?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::string::String;
    use std::vec::Vec;

    use super::*;
    use crate::primitive::nibble::UpperHexDigits;
    use crate::{BerLength, BerTag, Field, PackNibbles, UpperHexEven};

    type BerHex = Field<UpperHexEven<0, 64>, BerLength, PackNibbles<UpperHexDigits>>;
    type Ber = TlvList<BerTag, BerHex>;
    type PaddedBer = TlvList<BerTag, BerHex, PaddingByte<0x00>>;
    type Pairs = Vec<(String, String)>;
    type Map = BTreeMap<String, String>;

    fn pairs(entries: &[(&str, &str)]) -> Pairs {
        entries.iter().map(|&(tag, value)| (tag.into(), value.into())).collect()
    }

    fn encode<F: FieldEncode<C>, C>(value: &C) -> Result<Vec<u8>, Error> {
        let mut output = [0; 64];
        crate::encode::<F, C>(&mut output, &mut [0; 64], value)
            .map(|used| output[..used].to_vec())
            .map_err(|error| error.kind)
    }

    fn decode<'a, F: FieldDecode<'a, C>, C>(wire: &'a [u8], scratch: &'a mut [u8]) -> Result<C, Error> {
        F::decode_field(&mut &*wire, &mut &mut *scratch).map_err(|error| error.kind)
    }

    #[test]
    fn lists_keep_order_and_repeats_in_a_sequence_and_the_last_in_a_map() {
        let entries = pairs(&[("59", "ABCD"), ("9F02", "1234"), ("59", "00FF")]);
        let wire = b"\x59\x02\xAB\xCD\x9F\x02\x02\x12\x34\x59\x02\x00\xFF";
        assert_eq!(encode::<Ber, _>(&entries).as_deref(), Ok(&wire[..]));
        assert_eq!(decode::<Ber, Pairs>(wire, &mut [0; 64]), Ok(entries));
        let map = Map::from([("59".into(), "00FF".into()), ("9F02".into(), "1234".into())]);
        assert_eq!(decode::<Ber, Map>(wire, &mut [0; 64]), Ok(map.clone()));
        assert_eq!(encode::<Ber, _>(&map).as_deref(), Ok(&b"\x59\x02\x00\xFF\x9F\x02\x02\x12\x34"[..]));
        assert_eq!(decode::<Ber, Pairs>(b"", &mut []), Ok(Pairs::new()));
        // Each value's text stays in scratch, as values may borrow it; tags do not.
        assert!(decode::<Ber, Pairs>(wire, &mut [0; 12]).is_ok());
        assert_eq!(decode::<Ber, Pairs>(wire, &mut [0; 11]), Err(Error::BufferOverflow));
        assert_eq!(encode::<Ber, _>(&pairs(&[("bad", "12")])), Err(Error::Invalid));
        assert_eq!(encode::<Ber, _>(&pairs(&[("9F02", "12fg")])), Err(Error::Invalid));
    }

    #[test]
    fn padding_is_opt_in_skipped_around_entries_and_never_written() {
        let wire = b"\0\x59\x02\0\xFF\0\0\xFF\x01\0\0";
        let entries = pairs(&[("59", "00FF"), ("FF01", "")]);
        assert_eq!(decode::<PaddedBer, Pairs>(wire, &mut [0; 64]), Ok(entries.clone()));
        assert_eq!(decode::<Ber, Pairs>(wire, &mut [0; 64]), Err(Error::Invalid));
        assert_eq!(encode::<PaddedBer, _>(&entries).as_deref(), Ok(&b"\x59\x02\0\xFF\xFF\x01\0"[..]));
        assert_eq!(decode::<PaddedBer, Pairs>(b"\0\0", &mut []), Ok(Pairs::new()));
        for (wire, error) in [
            (&b"\0\x59\x02\0"[..], Error::UnexpectedEof),
            (b"\0\x59\x80\0", Error::Invalid),
            (b"\0\xFF", Error::UnexpectedEof),
        ] {
            assert_eq!(decode::<PaddedBer, Pairs>(wire, &mut [0; 64]), Err(error));
        }
    }
}

#[cfg(test)]
mod proptests {
    use std::string::String;
    use std::vec::Vec;

    use proptest::prelude::*;

    use super::*;
    use crate::primitive::nibble::UpperHexDigits;
    use crate::{BerLength, BerTag, Field, PackNibbles, UpperHexEven};

    type BerHex = Field<UpperHexEven<0, 64>, BerLength, PackNibbles<UpperHexDigits>>;
    type Ber = TlvList<BerTag, BerHex>;
    type PaddedBer = TlvList<BerTag, BerHex, PaddingByte<0x00>>;

    proptest! {
        #[test]
        fn padding_is_skipped_only_at_entry_boundaries(bytes in prop::collection::vec(any::<u8>(), 0..32), padding in prop::array::uniform3(0usize..8)) {
            let mut wire = vec![0; padding[0]];
            wire.extend_from_slice(&[0x59, bytes.len() as u8]);
            wire.extend_from_slice(&bytes);
            wire.extend(std::iter::repeat_n(0, padding[1]));
            wire.extend_from_slice(&[0x59, 0]);
            wire.extend(std::iter::repeat_n(0, padding[2]));
            let hex: String = bytes.iter().map(|byte| format!("{byte:02X}")).collect();
            let expected: Vec<(String, String)> = vec![("59".into(), hex), ("59".into(), "".into())];
            let (mut input, mut scratch) = (wire.as_slice(), [0; 128]);
            prop_assert_eq!(PaddedBer::decode_field(&mut input, &mut &mut scratch[..]).map_err(|error| error.kind), Ok(expected.clone()));
            prop_assert!(input.is_empty());
            let strict = Ber::decode_field(&mut wire.as_slice(), &mut &mut scratch[..]).map_err(|error| error.kind);
            prop_assert_eq!(strict, if padding == [0; 3] { Ok(expected) } else { Err(Error::Invalid) });
        }
    }
}
