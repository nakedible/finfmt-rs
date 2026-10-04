//! Tagged records: entries of a tag in a fixed-width scalar format followed
//! by a value whose format, declared per tag, carries its own length.

use core::str::FromStr;

use super::*;
use crate::utils::cold_path;

/// The longest tag a TLV record reads, in decoded bytes; a longer one is
/// `Invalid`.
#[doc(hidden)]
pub const MAX_TLV_TAG: usize = 16;

/// The unknown entries of a TLV record, collected by its `extras` field: the
/// tag's text as the key, and the value read by the field's `fmt`.
///
/// Implemented for collections of `(key, value)` pairs, such as a
/// `BTreeMap<String, String>`, which keeps the last value of a repeated tag.
pub trait TlvExtras<V> {
    /// Encode the entries, each tag through `T` and value through `F`,
    /// rejecting tags in `known_tags`, the record's declared tags, even if
    /// their fields are absent.
    fn encode_unknowns<T: ScalarFmt, F: FieldEncode<V>>(
        &self,
        output: &mut &mut [u8],
        scratch: &mut [u8],
        known_tags: &[&str],
    ) -> Result<(), CompositeError>;

    /// Decode the value of an entry whose tag is not declared, through `F`.
    fn decode_unknown<'de, F: FieldDecode<'de, V>>(
        &mut self,
        tag: &[u8],
        input: &mut &'de [u8],
        scratch: &mut &'de mut [u8],
    ) -> Result<(), CompositeError>;
}

impl<C, K, V> TlvExtras<V> for C
where
    C: Extend<(K, V)>,
    for<'a> &'a C: IntoIterator<Item = (&'a K, &'a V)>,
    K: AsRef<str> + FromStr,
{
    #[inline(always)]
    fn encode_unknowns<T: ScalarFmt, F: FieldEncode<V>>(
        &self,
        output: &mut &mut [u8],
        scratch: &mut [u8],
        known_tags: &[&str],
    ) -> Result<(), CompositeError> {
        for (key, value) in self {
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

/// Read the next entry's tag through `T` into `buf`, for an entry no declared
/// tag matched. The tag is only kept as text, so its scratch is not kept.
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
