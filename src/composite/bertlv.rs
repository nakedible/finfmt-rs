use core::str::FromStr;

#[cfg(all(not(debug_assertions), feature = "no-panic"))]
use no_panic::no_panic;

use super::*;
use crate::primitive::bertlv::{MAX_BER_TAG_BYTES, encode_ber_tlv_head, parse_ber_tag_hex};
use crate::primitive::bytes::reserve_bytes;
use crate::primitive::nibble::{UpperHexDigits, pack_nibbles_checked, unpack_nibbles};
use crate::utils::cold_path;

#[inline(always)]
fn decode_unknown_entry<K, V>(tag: &[u8], value: &[u8], scratch: &mut &mut [u8]) -> Result<(K, V), Error>
where
    K: FromStr,
    V: FromStr,
{
    // Both are parsed into owned values, so they reuse the same scratch.
    let key = encode_unknown_tag_key(&mut &mut **scratch, tag)?;
    let key = key.parse::<K>().map_err(|_| {
        crate::utils::cold_path();
        Error::Invalid
    })?;
    let value = encode_hex_upper(&mut &mut **scratch, value)?;
    let value = value.parse::<V>().map_err(|_| {
        crate::utils::cold_path();
        Error::Invalid
    })?;
    Ok((key, value))
}

#[inline(always)]
fn encode_unknown_entry<K: AsRef<str> + ?Sized, V: AsRef<str> + ?Sized>(
    output: &mut &mut [u8],
    key: &K,
    value: &V,
    known_tags: &[&str],
) -> Result<(), Error> {
    encode_unknown_tlv_from_key(output, key.as_ref(), value.as_ref(), known_tags)
}

impl<T, K, V> BerTlvExtras for T
where
    T: Extend<(K, V)>,
    for<'a> &'a T: IntoIterator<Item = (&'a K, &'a V)>,
    K: AsRef<str> + FromStr,
    V: AsRef<str> + FromStr,
{
    #[inline(always)]
    fn encode_unknowns(&self, output: &mut &mut [u8], _scratch: &mut [u8], known_tags: &[&str]) -> Result<(), Error> {
        for (key, value) in self {
            encode_unknown_entry(output, key, value, known_tags)?;
        }
        Ok(())
    }

    #[inline(always)]
    fn decode_unknown(&mut self, tag: &[u8], value: &[u8], scratch: &mut &mut [u8]) -> Result<(), Error> {
        // A repeated unknown tag is left to the collection: a map keeps the last value.
        let (key, value) = decode_unknown_entry::<K, V>(tag, value, scratch)?;
        self.extend(core::iter::once((key, value)));
        Ok(())
    }
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub(super) fn encode_hex_upper<'a>(scratch: &mut &'a mut [u8], bytes: &[u8]) -> Result<&'a str, Error> {
    let out = unpack_nibbles::<UpperHexDigits>(scratch, bytes)?;
    core::str::from_utf8(out).map_err(|_| {
        cold_path();
        Error::Internal
    })
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub(super) fn encode_unknown_tag_key<'a>(scratch: &mut &'a mut [u8], tag: &[u8]) -> Result<&'a str, Error> {
    let needed = tag
        .len()
        .checked_mul(2)
        .and_then(|len| len.checked_add(1 + "_unknown".len()))
        .ok_or_else(|| {
            cold_path();
            Error::BufferOverflow
        })?;
    let out = reserve_bytes(scratch, needed)?;
    let mut cursor = &mut *out;
    copy_bytes(&mut cursor, b"t")?;
    unpack_nibbles::<UpperHexDigits>(&mut cursor, tag)?;
    copy_bytes(&mut cursor, b"_unknown")?;
    core::str::from_utf8(out).map_err(|_| {
        cold_path();
        Error::Internal
    })
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub(super) fn parse_unknown_tag_key(key: &str) -> Result<([u8; MAX_BER_TAG_BYTES], usize), Error> {
    let body = key
        .strip_prefix('t')
        .and_then(|rest| rest.strip_suffix("_unknown"))
        .ok_or_else(|| {
            cold_path();
            Error::Invalid
        })?;
    parse_ber_tag_hex(body)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub(super) fn encode_unknown_tlv_from_key(output: &mut &mut [u8], key: &str, value: &str, known_tags: &[&str]) -> Result<(), Error> {
    let (tag_bytes, tag_len) = parse_unknown_tag_key(key)?;
    let tag = tag_bytes.get(..tag_len).unwrap_or(&tag_bytes);
    // A parsed key is canonical uppercase hex, as are the declared literals.
    let tag_hex = key
        .strip_prefix('t')
        .and_then(|rest| rest.strip_suffix("_unknown"))
        .unwrap_or_default();
    if known_tags.contains(&tag_hex) {
        cold_path();
        return Err(Error::Invalid);
    }
    encode_unknown_tlv_from_tag(output, tag, value)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub(super) fn encode_unknown_tlv_from_tag(output: &mut &mut [u8], tag: &[u8], value: &str) -> Result<(), Error> {
    encode_ber_tlv_head(output, tag, value.len() / 2)?;
    pack_nibbles_checked::<UpperHexDigits>(output, value.as_bytes())?;
    Ok(())
}

/// Frame the next entry, first skipping `00` padding when allowed. EMV 4.4 Book 3
/// Annex B: "Before, between, or after TLV-coded data objects, '00' bytes
/// without any meaning may occur".
#[doc(hidden)]
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn decode_ber_tlv_collection_entry<'a, const ALLOW_ZERO_PADDING: bool>(
    input: &mut &'a [u8],
) -> Result<Option<crate::primitive::bertlv::BerTlvEntry<'a>>, Error> {
    if ALLOW_ZERO_PADDING {
        *input = crate::primitive::text::decode_padded(input, 0, true, 0);
    }
    crate::primitive::bertlv::decode_ber_tlv_entry(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitive::bertlv::{BerTlvEntry, MAX_BER_VALUE_LEN, ber_length_width, decode_ber_tlv_entry};

    #[test]
    fn test_parse_unknown_tag_key_uppercase_only() {
        assert_eq!(parse_unknown_tag_key("t9F02_unknown"), Ok(([0x9F, 0x02, 0, 0], 2)));
        assert_eq!(parse_unknown_tag_key("t00_unknown"), Err(Error::Invalid));
        assert_eq!(parse_unknown_tag_key("t9f02_unknown"), Err(Error::Invalid));
        assert_eq!(parse_unknown_tag_key("9F02_unknown"), Err(Error::Invalid));
        assert_eq!(parse_unknown_tag_key("t9F02"), Err(Error::Invalid));
    }

    #[test]
    fn test_unknown_encoding_boundaries() {
        for len in [0, 1, 127, 128, 255, 256, MAX_BER_VALUE_LEN] {
            let value = "AB".repeat(len);
            let total = 2 + ber_length_width(len).unwrap() + len;
            let mut storage = vec![0xEE; total + 1];
            for capacity in [0, 1, 2, total - 1, total, total + 1] {
                let mut output = &mut storage[..capacity];
                let result = encode_unknown_tlv_from_key(&mut output, "t9F02_unknown", &value, &[]);
                if capacity < total {
                    assert_eq!(result, Err(Error::BufferOverflow));
                } else {
                    assert_eq!(result, Ok(()));
                    assert_eq!(output.len(), capacity - total);
                    let mut wire = &storage[..total];
                    assert_eq!(
                        decode_ber_tlv_entry(&mut wire),
                        Ok(Some(BerTlvEntry {
                            tag: &[0x9F, 0x02],
                            value: &vec![0xAB; len]
                        }))
                    );
                    assert!(wire.is_empty());
                }
            }
        }
        let mut output = [0u8; 16];
        for key in [
            "t00_unknown",
            "t9F_unknown",
            "t5A5B_unknown",
            "t9f02_unknown",
            "t9F02",
            "9F02_unknown",
        ] {
            assert_eq!(encode_unknown_tlv_from_key(&mut &mut output[..], key, "", &[]), Err(Error::Invalid));
        }
        for (value, error) in [("aB", Error::Invalid), ("GG", Error::Invalid), ("A", Error::Invalid)] {
            assert_eq!(
                encode_unknown_tlv_from_key(&mut &mut output[..], "t5A_unknown", value, &[]),
                Err(error)
            );
        }
        assert_eq!(
            encode_unknown_tlv_from_key(&mut &mut [][..], "t5A_unknown", &"00".repeat(MAX_BER_VALUE_LEN + 1), &[]),
            Err(Error::Invalid)
        );
    }

    #[test]
    fn textual_formatting_uses_exact_scratch_and_canonical_hex() {
        for (bytes, expected) in [(b"".as_slice(), ""), (b"\0\xFF", "00FF"), (b"\x12\xAB", "12AB")] {
            let mut storage = [0xEE; 16];
            let mut scratch = storage.as_mut_slice();
            assert_eq!(encode_hex_upper(&mut scratch, bytes), Ok(expected));
            assert_eq!(scratch, &[0xEE; 16][expected.len()..]);
            assert_eq!(encode_hex_upper(&mut &mut storage[..expected.len()], bytes), Ok(expected));
            if !expected.is_empty() {
                assert_eq!(
                    encode_hex_upper(&mut &mut storage[..expected.len() - 1], bytes),
                    Err(Error::BufferOverflow)
                );
            }
        }
        for text in ["5A", "9F02", "9F8101", "9F818001"] {
            let (tag, len) = parse_ber_tag_hex(text).unwrap();
            let expected = format!("t{text}_unknown");
            let mut storage = [0xEE; 32];
            for capacity in [expected.len() - 1, expected.len(), storage.len()] {
                let mut scratch = &mut storage[..capacity];
                let result = encode_unknown_tag_key(&mut scratch, &tag[..len]);
                if capacity < expected.len() {
                    assert_eq!(result, Err(Error::BufferOverflow));
                } else {
                    let key = result.unwrap();
                    assert_eq!(key, expected);
                    assert_eq!(parse_unknown_tag_key(key), Ok((tag, len)));
                    assert_eq!(scratch, &[0xEE; 32][..capacity - expected.len()]);
                }
            }
        }
        for key in ["t_unknown", "T5A_unknown", "t5A_UNKNOWN", "t5_unknown", "t5A_unknown_more"] {
            assert_eq!(parse_unknown_tag_key(key), Err(Error::Invalid));
        }
    }
}

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[test]
        fn formatted_values_roundtrip(bytes in prop::collection::vec(any::<u8>(), 0..64)) {
            let expected: String = bytes.iter().map(|byte| format!("{byte:02X}")).collect();
            let mut storage = [0; 128];
            let text = encode_hex_upper(&mut storage.as_mut_slice(), &bytes).unwrap();
            prop_assert_eq!(text, expected);
            let mut output = [0; 128];
            let mut cursor = output.as_mut_slice();
            encode_unknown_tlv_from_key(&mut cursor, "t5A_unknown", text, &[]).unwrap();
            let used = 128 - cursor.len();
            let mut input = &output[..used];
            let entry = crate::primitive::bertlv::decode_ber_tlv_entry(&mut input).unwrap().unwrap();
            prop_assert_eq!(entry.tag, &[0x5A]);
            prop_assert_eq!(entry.value, bytes);
            prop_assert!(input.is_empty());
        }
    }
}
