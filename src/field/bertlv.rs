//! BER-TLV tags and lengths as field formats, for `tlv` records.

use crate::primitive::bertlv::{ber_length_width, decode_ber_length, decode_ber_tag, encode_ber_length, parse_ber_tag_hex};
use crate::primitive::bytes::copy_bytes;
use crate::primitive::nibble::{UpperHexDigits, unpack_nibbles};
use crate::{Error, LengthSpec, ScalarFmt};

/// A BER tag, one to four bytes by the BER tag rules, as uppercase hex text
/// such as `"9F02"`: the tag format of a BER-TLV record,
/// `#[wire(tlv(tag = BerTag, padding = 0x00))]`.
///
/// Tag numbers padded with leading zero bits, such as `DF8002`, are accepted,
/// as some specifications use them; [`StrictBerTag`] rejects them, so each
/// tag has one encoding.
pub struct BerTag<const STRICT: bool = false>;

/// [`BerTag`] that rejects tag numbers padded with leading zero bits.
pub type StrictBerTag = BerTag<true>;

impl<const STRICT: bool> ScalarFmt for BerTag<STRICT> {
    #[inline(always)]
    fn encoded_len(input: &[u8]) -> Result<usize, Error> {
        let (_, len) = parse_ber_tag_hex::<STRICT>(core::str::from_utf8(input).map_err(|_| Error::Invalid)?)?;
        Ok(len)
    }

    #[inline(always)]
    fn encode(output: &mut &mut [u8], _scratch: &mut [u8], input: &[u8]) -> Result<(), Error> {
        let (tag, len) = parse_ber_tag_hex::<STRICT>(core::str::from_utf8(input).map_err(|_| Error::Invalid)?)?;
        copy_bytes(output, tag.get(..len).unwrap_or_default())?;
        Ok(())
    }

    #[inline(always)]
    fn decode<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<&'a [u8], Error> {
        let tag = decode_ber_tag::<STRICT>(input)?;
        Ok(unpack_nibbles::<UpperHexDigits>(scratch, tag)?)
    }
}

/// A definite BER length, in the short or long form: the length of a
/// BER-TLV value, such as `Field<UpperHexEven<0, 64>, BerLength,
/// PackNibbles<UpperHexDigits>>` or `Frame<BerLength, Record>`. Decoding
/// accepts the long form for short lengths; encoding writes the shortest.
pub struct BerLength;

impl LengthSpec for BerLength {
    #[inline(always)]
    fn encoded_len(len: usize) -> Result<usize, Error> {
        ber_length_width(len)
    }

    #[inline(always)]
    fn encode(output: &mut &mut [u8], _scratch: &mut [u8], len: usize) -> Result<(), Error> {
        encode_ber_length(output, len)?;
        Ok(())
    }

    #[inline(always)]
    fn decode<'a>(input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<Option<usize>, Error> {
        decode_ber_length(input).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ber_tags_and_lengths_round_trip() {
        for (text, wire) in [("5A", &[0x5A][..]), ("9F02", &[0x9F, 0x02]), ("DF8102", &[0xDF, 0x81, 0x02])] {
            let mut output = [0; 4];
            let mut cursor = &mut output[..];
            <BerTag>::encode(&mut cursor, &mut [], text.as_bytes()).unwrap();
            assert_eq!(&output[..wire.len()], wire);
            let mut scratch = [0; 8];
            assert_eq!(<BerTag>::decode(&mut &wire[..], &mut &mut scratch[..]), Ok(text.as_bytes()));
        }
        assert_eq!(<BerTag>::encode(&mut &mut [0; 4][..], &mut [], b"9F"), Err(Error::Invalid));
        // A tag number padded with a leading zero group: accepted unless strict.
        let mut scratch = [0; 8];
        assert_eq!(
            BerTag::<false>::decode(&mut &b"\xDF\x80\x02"[..], &mut &mut scratch[..]),
            Ok(&b"DF8002"[..])
        );
        assert_eq!(
            StrictBerTag::decode(&mut &b"\xDF\x80\x02"[..], &mut &mut [0; 8][..]),
            Err(Error::Invalid)
        );
        assert_eq!(StrictBerTag::encode(&mut &mut [0; 4][..], &mut [], b"DF8002"), Err(Error::Invalid));
        for (len, wire) in [(5, &[0x05][..]), (200, &[0x81, 200]), (300, &[0x82, 0x01, 0x2C])] {
            let mut output = [0; 3];
            let mut cursor = &mut output[..];
            BerLength::encode(&mut cursor, &mut [], len).unwrap();
            assert_eq!(&output[..wire.len()], wire);
            assert_eq!(BerLength::decode(&mut &wire[..], &mut &mut [][..]), Ok(Some(len)));
        }
    }
}
