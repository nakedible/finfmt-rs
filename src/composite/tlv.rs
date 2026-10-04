//! Generic tag-length-value records: entries of a tag in a fixed-width scalar
//! format, a length in any [`LengthSpec`], and the value it counts.

use core::str::FromStr;

use super::bertlv::encode_hex_upper;
use super::*;
use crate::field::{Identity, LengthSpec, decode_framed};
use crate::primitive::nibble::{UpperHexDigits, pack_nibbles_checked};
use crate::utils::cold_path;

/// The longest tag a TLV record reads, in decoded bytes; a longer one is
/// `Invalid`.
#[doc(hidden)]
pub const MAX_TLV_TAG: usize = 16;

/// The unknown entries of a TLV record, collected by its `extras` field: the
/// tag's text and the value as uppercase hex.
///
/// Implemented for collections of `(key, value)` pairs, such as a
/// `BTreeMap<String, String>`, which keeps the last value of a repeated tag.
pub trait TlvExtras {
    /// Encode the entries with the record's tag and length formats,
    /// rejecting tags in `known_tags`, the record's declared tags, even if
    /// their fields are absent.
    fn encode_unknowns<T: ScalarFmt, L: LengthSpec>(
        &self,
        output: &mut &mut [u8],
        scratch: &mut [u8],
        known_tags: &[&str],
    ) -> Result<(), Error>;
    fn decode_unknown(&mut self, tag: &[u8], value: &[u8], scratch: &mut &mut [u8]) -> Result<(), Error>;
}

impl<C, K, V> TlvExtras for C
where
    C: Extend<(K, V)>,
    for<'a> &'a C: IntoIterator<Item = (&'a K, &'a V)>,
    K: AsRef<str> + FromStr,
    V: AsRef<str> + FromStr,
{
    #[inline(always)]
    fn encode_unknowns<T: ScalarFmt, L: LengthSpec>(
        &self,
        output: &mut &mut [u8],
        scratch: &mut [u8],
        known_tags: &[&str],
    ) -> Result<(), Error> {
        for (key, value) in self {
            let key = key.as_ref();
            if known_tags.contains(&key) {
                cold_path();
                return Err(Error::Invalid);
            }
            let (bytes, scratch) = encode_staged::<Error, _>(&mut *scratch, |out, _| {
                pack_nibbles_checked::<UpperHexDigits>(out, value.as_ref().as_bytes()).map(|_| ())
            })?;
            encode_tlv_raw::<T, L>(output, scratch, key, bytes)?;
        }
        Ok(())
    }

    #[inline(always)]
    fn decode_unknown(&mut self, tag: &[u8], value: &[u8], scratch: &mut &mut [u8]) -> Result<(), Error> {
        let key = core::str::from_utf8(tag)
            .ok()
            .and_then(|tag| tag.parse::<K>().ok())
            .ok_or_else(|| {
                cold_path();
                Error::Invalid
            })?;
        // The value is parsed into an owned value, so its hex reuses scratch.
        let value = encode_hex_upper(&mut &mut **scratch, value)?.parse::<V>().map_err(|_| {
            cold_path();
            Error::Invalid
        })?;
        self.extend(core::iter::once((key, value)));
        Ok(())
    }
}

/// Write one entry: the tag through `T`, the value's length through `L`, then
/// the value.
#[inline(always)]
fn encode_tlv_raw<T: ScalarFmt, L: LengthSpec>(output: &mut &mut [u8], scratch: &mut [u8], tag: &str, value: &[u8]) -> Result<(), Error> {
    T::encode(output, scratch, tag.as_bytes())?;
    // The length comes from the value: one the prefix cannot state is the value's.
    L::encode(output, scratch, value.len()).map_err(|error| match error {
        Error::Invalid => Error::InvalidValueLength,
        error => error,
    })?;
    copy_bytes(output, value)?;
    Ok(())
}

/// Encode a declared field as one entry, staging its value so the length can
/// precede it.
#[doc(hidden)]
#[inline(always)]
pub fn encode_tlv_field<T, L, F>(
    output: &mut &mut [u8],
    scratch: &mut [u8],
    tag: &str,
    field: &'static str,
    encode_value: F,
) -> Result<(), CompositeError>
where
    T: ScalarFmt,
    L: LengthSpec,
    F: FnOnce(&mut &mut [u8], &mut [u8]) -> Result<(), CompositeError>,
{
    let (value, scratch) = encode_staged(scratch, encode_value).map_err(|error| wrap_composite_error(error, field))?;
    let result = encode_tlv_raw::<T, L>(output, scratch, tag, value);
    debug_assert!(
        result != Err(Error::Invalid),
        "a declared tag is not a valid value of the record's tag format"
    );
    result.map_err(|error| wrap_composite_error(error, field))
}

/// Read the next entry's tag through `T` into `buf`. The tag is only
/// compared, so its scratch is not kept.
#[doc(hidden)]
#[inline(always)]
pub fn decode_tlv_tag<'b, T: ScalarFmt>(input: &mut &[u8], scratch: &mut [u8], buf: &'b mut [u8; MAX_TLV_TAG]) -> Result<&'b [u8], Error> {
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

/// Read the length `L` states and take the value it counts.
#[doc(hidden)]
#[inline(always)]
pub fn decode_tlv_value<'a, L: LengthSpec>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<&'a [u8], Error> {
    decode_framed::<L, Identity>(input, scratch)
}

/// Decode a declared field's value, which must use all of it. A tag seen
/// twice is `Invalid`.
#[doc(hidden)]
#[inline(always)]
pub fn decode_tlv_field<'a, L, T, D>(
    input: &mut &'a [u8],
    scratch: &mut &'a mut [u8],
    field_value: &mut Option<T>,
    field: &'static str,
    decode_value: D,
) -> Result<(), CompositeError>
where
    L: LengthSpec,
    D: FnOnce(&mut &'a [u8], &mut &'a mut [u8]) -> Result<T, CompositeError>,
{
    if field_value.is_some() {
        cold_path();
        return Err(wrap_composite_error(Error::Invalid, field));
    }
    let mut value_input = decode_tlv_value::<L>(input, scratch).map_err(|error| wrap_composite_error(error, field))?;
    let value = decode_value(&mut value_input, scratch).map_err(|error| wrap_composite_error(error, field))?;
    if !value_input.is_empty() {
        cold_path();
        return Err(wrap_composite_error(Error::Invalid, field));
    }
    *field_value = Some(value);
    Ok(())
}
