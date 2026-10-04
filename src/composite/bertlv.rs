#[cfg(all(not(debug_assertions), feature = "no-panic"))]
use no_panic::no_panic;

use super::*;
use crate::primitive::bertlv::encode_ber_tlv_head;
use crate::primitive::nibble::{UpperHexDigits, pack_nibbles_checked, unpack_nibbles};
use crate::utils::cold_path;

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
pub(super) fn encode_unknown_tlv_from_tag(output: &mut &mut [u8], tag: &[u8], value: &str) -> Result<(), Error> {
    // Hex text of odd length is fixed by adding or removing a digit.
    if !value.len().is_multiple_of(2) {
        cold_path();
        return Err(Error::InvalidValueLength);
    }
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
pub(super) fn decode_ber_tlv_collection_entry<'a, const ALLOW_ZERO_PADDING: bool>(
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
    fn test_unknown_encoding_boundaries() {
        for len in [0, 1, 127, 128, 255, 256, MAX_BER_VALUE_LEN] {
            let value = "AB".repeat(len);
            let total = 2 + ber_length_width(len).unwrap() + len;
            let mut storage = vec![0xEE; total + 1];
            for capacity in [0, 1, 2, total - 1, total, total + 1] {
                let mut output = &mut storage[..capacity];
                let result = encode_unknown_tlv_from_tag(&mut output, &[0x9F, 0x02], &value);
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
        for (value, error) in [("aB", Error::Invalid), ("GG", Error::Invalid), ("A", Error::InvalidValueLength)] {
            assert_eq!(encode_unknown_tlv_from_tag(&mut &mut output[..], &[0x5A], value), Err(error));
        }
        assert_eq!(
            encode_unknown_tlv_from_tag(&mut &mut [][..], &[0x5A], &"00".repeat(MAX_BER_VALUE_LEN + 1)),
            Err(Error::InvalidValueLength)
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
            encode_unknown_tlv_from_tag(&mut cursor, &[0x5A], text).unwrap();
            let used = 128 - cursor.len();
            let mut input = &output[..used];
            let entry = crate::primitive::bertlv::decode_ber_tlv_entry(&mut input).unwrap().unwrap();
            prop_assert_eq!(entry.tag, &[0x5A]);
            prop_assert_eq!(entry.value, bytes);
            prop_assert!(input.is_empty());
        }
    }
}
