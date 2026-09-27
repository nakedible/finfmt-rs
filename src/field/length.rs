use core::marker::PhantomData;

use crate::primitive::decimal::{
    decode_ascii_decimal_fixed, decode_ebcdic_decimal_blank_zero_fixed, decode_ebcdic_decimal_fixed, encode_ascii_decimal_fixed,
    encode_ebcdic_decimal_blank_zero_fixed, encode_ebcdic_decimal_fixed, fits_decimal_width,
};
use crate::utils::cold_path;
use crate::{Error, ScalarFmt};

#[inline(always)]
fn decimal_prefix_len(value: usize, width: usize) -> Result<usize, Error> {
    if !fits_decimal_width(value, width) {
        cold_path();
        return Err(Error::Invalid);
    }
    Ok(width)
}

/// How a value states its length: a prefix, a fixed length, or nothing.
///
/// The length is a number of units of the framed data. A [`crate::Field`]
/// counts at its [`crate::Count`] marker, or its wire bytes without one; a
/// [`crate::composite::BoundedList`] counts items. The spec holds the number's
/// codec and its arithmetic ([`Offset`], [`Per`]), the same for every consumer.
pub trait LengthSpec {
    /// The framing states a length. Only [`Rest`] does not: it takes whatever
    /// input the enclosing format supplies.
    const STATES_LEN: bool = true;

    /// Encoded prefix size for the length `len`, after checking it fits.
    fn encoded_len(len: usize) -> Result<usize, Error>;
    /// Write the prefix for the length `len`, if any.
    fn encode(output: &mut &mut [u8], scratch: &mut [u8], len: usize) -> Result<(), Error>;
    /// Read the prefix, if any, and return the length it states: `None` for
    /// [`Rest`].
    fn decode<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<Option<usize>, Error>;
}

/// A length of `N` units with no prefix. `Fixed<8>` is 8 wire bytes; with the
/// marker in `chain!(PadLeft<19, b'0'>, Count, PackNibblesRight<…>)`,
/// `Fixed<19>` is 19 digits in 10 bytes. Encoding checks in debug builds that
/// the value has that length.
pub struct Fixed<const N: usize>;

impl<const N: usize> LengthSpec for Fixed<N> {
    #[inline(always)]
    fn encoded_len(len: usize) -> Result<usize, Error> {
        debug_assert_eq!(len, N, "value does not have the fixed length");
        Ok(0)
    }

    #[inline(always)]
    fn encode(_output: &mut &mut [u8], _scratch: &mut [u8], len: usize) -> Result<(), Error> {
        <Self as LengthSpec>::encoded_len(len)?;
        Ok(())
    }

    #[inline(always)]
    fn decode<'a>(_input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<Option<usize>, Error> {
        Ok(Some(N))
    }
}

/// A prefix in the scalar format `F`.
pub struct Length<F>(PhantomData<F>);

impl<F: ScalarFmt> LengthSpec for Length<F> {
    #[inline(always)]
    fn encoded_len(len: usize) -> Result<usize, Error> {
        F::encoded_len_usize(len)
    }

    #[inline(always)]
    fn encode(output: &mut &mut [u8], scratch: &mut [u8], len: usize) -> Result<(), Error> {
        F::encode_usize(output, scratch, len)
    }

    #[inline(always)]
    fn decode<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<Option<usize>, Error> {
        F::decode_usize(input, scratch).map(Some)
    }
}

/// A prefix of `N` ASCII decimal digits.
pub struct AsciiLength<const N: usize>;

impl<const N: usize> LengthSpec for AsciiLength<N> {
    #[inline(always)]
    fn encoded_len(len: usize) -> Result<usize, Error> {
        decimal_prefix_len(len, N)
    }

    #[inline(always)]
    fn encode(output: &mut &mut [u8], _scratch: &mut [u8], len: usize) -> Result<(), Error> {
        encode_ascii_decimal_fixed(output, len, N)
    }

    #[inline(always)]
    fn decode<'a>(input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<Option<usize>, Error> {
        decode_ascii_decimal_fixed(input, N).map(Some)
    }
}

/// A prefix of `N` EBCDIC decimal digits.
pub struct EbcdicLength<const N: usize>;

impl<const N: usize> LengthSpec for EbcdicLength<N> {
    #[inline(always)]
    fn encoded_len(len: usize) -> Result<usize, Error> {
        decimal_prefix_len(len, N)
    }

    #[inline(always)]
    fn encode(output: &mut &mut [u8], _scratch: &mut [u8], len: usize) -> Result<(), Error> {
        encode_ebcdic_decimal_fixed(output, len, N)
    }

    #[inline(always)]
    fn decode<'a>(input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<Option<usize>, Error> {
        decode_ebcdic_decimal_fixed(input, N).map(Some)
    }
}

/// [`EbcdicLength`] that encodes zero as `N` EBCDIC blanks and decodes
/// either blanks or zero digits as zero.
pub struct BlankableEbcdicLength<const N: usize>;

impl<const N: usize> LengthSpec for BlankableEbcdicLength<N> {
    #[inline(always)]
    fn encoded_len(len: usize) -> Result<usize, Error> {
        decimal_prefix_len(len, N)
    }

    #[inline(always)]
    fn encode(output: &mut &mut [u8], _scratch: &mut [u8], len: usize) -> Result<(), Error> {
        encode_ebcdic_decimal_blank_zero_fixed(output, len, N)
    }

    #[inline(always)]
    fn decode<'a>(input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<Option<usize>, Error> {
        decode_ebcdic_decimal_blank_zero_fixed(input, N).map(Some)
    }
}

/// A length whose prefix `L` states it plus `K`, for lengths that also count
/// bytes or items outside what they frame: the prefix itself, a header before
/// it, or a header item. An IBM RDW is `Offset<Length<FixedBinaryBe<2>>, 2>`
/// framing its two reserved bytes and the record. A stated value below `K` is
/// `Invalid`.
pub struct Offset<L, const K: usize>(PhantomData<L>);

#[inline(always)]
fn add_offset<const K: usize>(len: usize) -> Result<usize, Error> {
    len.checked_add(K).ok_or_else(|| {
        cold_path();
        Error::Invalid
    })
}

impl<L: LengthSpec, const K: usize> LengthSpec for Offset<L, K> {
    const STATES_LEN: bool = L::STATES_LEN;

    #[inline(always)]
    fn encoded_len(len: usize) -> Result<usize, Error> {
        L::encoded_len(add_offset::<K>(len)?)
    }

    #[inline(always)]
    fn encode(output: &mut &mut [u8], scratch: &mut [u8], len: usize) -> Result<(), Error> {
        L::encode(output, scratch, add_offset::<K>(len)?)
    }

    #[inline(always)]
    fn decode<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<Option<usize>, Error> {
        let Some(stated) = L::decode(input, scratch)? else {
            return Ok(None);
        };
        stated.checked_sub(K).map(Some).ok_or_else(|| {
            cold_path();
            Error::Invalid
        })
    }
}

/// A length whose prefix `L` counts groups of `D` units: binary data carried
/// as hex text, counted in bytes, is `Per<L, 2>` over the hex digits. Encoding
/// a length that is not a multiple of `D` is a composition mistake, asserted
/// in debug builds; the value's check must rule it out. A stated count whose
/// length overflows is `Invalid`.
pub struct Per<L, const D: usize>(PhantomData<L>);

impl<L: LengthSpec, const D: usize> LengthSpec for Per<L, D> {
    const STATES_LEN: bool = L::STATES_LEN;

    #[inline(always)]
    fn encoded_len(len: usize) -> Result<usize, Error> {
        const { assert!(D != 0, "a Per length needs a nonzero group size") };
        debug_assert!(len.is_multiple_of(D), "the length is not a whole number of groups");
        L::encoded_len(len / D)
    }

    #[inline(always)]
    fn encode(output: &mut &mut [u8], scratch: &mut [u8], len: usize) -> Result<(), Error> {
        const { assert!(D != 0, "a Per length needs a nonzero group size") };
        debug_assert!(len.is_multiple_of(D), "the length is not a whole number of groups");
        L::encode(output, scratch, len / D)
    }

    #[inline(always)]
    fn decode<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<Option<usize>, Error> {
        const { assert!(D != 0, "a Per length needs a nonzero group size") };
        let Some(groups) = L::decode(input, scratch)? else {
            return Ok(None);
        };
        groups.checked_mul(D).map(Some).ok_or_else(|| {
            cold_path();
            Error::Invalid
        })
    }
}

/// Consume the entire remaining input supplied by the enclosing format.
/// A following sibling requires an enclosing frame that bounds this input.
pub struct Rest;

impl LengthSpec for Rest {
    const STATES_LEN: bool = false;

    #[inline(always)]
    fn encoded_len(_len: usize) -> Result<usize, Error> {
        Ok(0)
    }

    #[inline(always)]
    fn encode(_output: &mut &mut [u8], _scratch: &mut [u8], _len: usize) -> Result<(), Error> {
        Ok(())
    }

    #[inline(always)]
    fn decode<'a>(_input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<Option<usize>, Error> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AsciiLength, BlankableEbcdicLength, EbcdicLength, Fixed, Length, LengthSpec, Offset, Per, Rest, encode_ascii_decimal_fixed,
    };
    use crate::field::{Ascii, Field, FixedBinaryBe};
    use crate::{Error, ScalarFmt};

    fn encode_length<L: LengthSpec>(len: usize) -> Result<Vec<u8>, Error> {
        let mut output = [0; 32];
        let mut out = output.as_mut_slice();
        let encoded = L::encode(&mut out, &mut [][..], len);
        let written = 32 - out.len();
        assert_eq!(L::encoded_len(len), encoded.map(|()| written));
        encoded.map(|()| output[..written].to_vec())
    }

    fn decode_length<L: LengthSpec>(input: &[u8]) -> Result<Option<usize>, Error> {
        let mut input = input;
        let result = L::decode(&mut input, &mut &mut [][..]);
        assert!(result.is_err() || input.is_empty());
        result
    }

    #[test]
    fn offset_prefixes_count_what_lies_outside_the_payload() {
        // An IBM RDW: a 2-byte length counting itself and two reserved bytes.
        type Rdw = Offset<Length<FixedBinaryBe<2>>, 2>;
        assert_eq!(encode_length::<Rdw>(10).unwrap(), [0x00, 0x0C]);
        assert_eq!(decode_length::<Rdw>(&[0x00, 0x0C]), Ok(Some(10)));
        for wire in [&[0x00, 0x01][..], &[0x00, 0x00]] {
            assert_eq!(decode_length::<Rdw>(wire), Err(Error::Invalid));
        }
        // A count that includes a header item.
        type Items = Offset<AsciiLength<5>, 1>;
        assert_eq!(encode_length::<Items>(2).unwrap(), b"00003");
        assert_eq!(decode_length::<Items>(b"00003"), Ok(Some(2)));
        assert_eq!(decode_length::<Items>(b"00000"), Err(Error::Invalid));
        assert_eq!(decode_length::<Offset<Rest, 4>>(b""), Ok(None));

        type Record = Field<crate::Binary<0, 20>, Rdw>;
        let mut output = [0; 8];
        Record::encode(&mut &mut output[..], &mut [], b"\0\0AB").unwrap();
        assert_eq!(output[..6], *b"\x00\x06\0\0AB");
        assert_eq!(Record::decode(&mut &output[..6], &mut &mut [][..]), Ok(&b"\0\0AB"[..]));
    }

    #[test]
    fn per_prefixes_count_groups() {
        type Bytes = Per<AsciiLength<2>, 2>;
        assert_eq!(encode_length::<Bytes>(8).unwrap(), b"04");
        assert_eq!(decode_length::<Bytes>(b"04"), Ok(Some(8)));
        assert_eq!(
            decode_length::<Per<Length<FixedBinaryBe<8>>, 2>>(&u64::MAX.to_be_bytes()),
            Err(Error::Invalid)
        );
        assert_eq!(decode_length::<Per<Rest, 2>>(b""), Ok(None));
        // Inside `Per`, the offset is in groups; outside, in the counted units.
        type Groups = Per<Offset<AsciiLength<2>, 1>, 2>;
        assert_eq!(encode_length::<Groups>(8).unwrap(), b"05");
        assert_eq!(decode_length::<Groups>(b"05"), Ok(Some(8)));
        type Units = Offset<Per<AsciiLength<2>, 2>, 2>;
        assert_eq!(encode_length::<Units>(8).unwrap(), b"05");
        assert_eq!(decode_length::<Units>(b"05"), Ok(Some(8)));
        if cfg!(debug_assertions) {
            assert!(std::panic::catch_unwind(|| encode_length::<Bytes>(3)).is_err());
        }
    }

    #[test]
    fn decimal_prefixes() {
        assert_eq!(encode_length::<AsciiLength<2>>(16).unwrap(), b"16");
        assert_eq!(encode_length::<AsciiLength<3>>(255).unwrap(), b"255");
        assert_eq!(decode_length::<AsciiLength<2>>(b"16"), Ok(Some(16)));
        assert_eq!(encode_length::<EbcdicLength<2>>(16).unwrap(), [0xF1, 0xF6]);
        assert_eq!(encode_length::<EbcdicLength<4>>(9999).unwrap(), [0xF9; 4]);
        assert_eq!(decode_length::<EbcdicLength<2>>(&[0xF1, 0xF6]), Ok(Some(16)));
        assert_eq!(encode_length::<BlankableEbcdicLength<2>>(0).unwrap(), [0x40, 0x40]);
        assert_eq!(encode_length::<BlankableEbcdicLength<2>>(16).unwrap(), [0xF1, 0xF6]);
        assert_eq!(decode_length::<BlankableEbcdicLength<2>>(&[0x40, 0x40]), Ok(Some(0)));
        assert_eq!(decode_length::<BlankableEbcdicLength<2>>(&[0xF1, 0xF6]), Ok(Some(16)));
        for wire in [&b"A0"[..], &[0xF0, b'0'], &[0x40, 0xF0]] {
            assert_eq!(decode_length::<BlankableEbcdicLength<2>>(wire), Err(Error::Invalid));
        }
        assert_eq!(decode_length::<AsciiLength<2>>(b"A0"), Err(Error::Invalid));
        for value in [0, 1, 9, 10, 99, 100, usize::MAX] {
            let expected = if value < 100 { Ok(2) } else { Err(Error::Invalid) };
            assert_eq!(encode_length::<AsciiLength<2>>(value).map(|v| v.len()), expected);
            assert_eq!(encode_length::<EbcdicLength<2>>(value).map(|v| v.len()), expected);
            assert_eq!(encode_length::<BlankableEbcdicLength<2>>(value).map(|v| v.len()), expected);
        }
        assert_eq!(encode_length::<AsciiLength<20>>(usize::MAX).map(|v| v.len()), Ok(20));
        assert_eq!(encode_length::<AsciiLength<32>>(usize::MAX).map(|v| v.len()), Ok(32));
        let mut ascii = [0; 20];
        encode_ascii_decimal_fixed(&mut &mut ascii[..], usize::MAX, 20).unwrap();
        assert_eq!(decode_length::<AsciiLength<20>>(&ascii), Ok(Some(usize::MAX)));
    }

    #[test]
    fn fixed_lengths_assert_only_in_debug() {
        for len in [0, 1, 3] {
            let predicted = std::panic::catch_unwind(|| Fixed::<2>::encoded_len(len));
            let encoded = std::panic::catch_unwind(|| Fixed::<2>::encode(&mut &mut [][..], &mut [][..], len));
            if cfg!(debug_assertions) {
                assert!(predicted.is_err() && encoded.is_err());
            } else {
                assert_eq!(predicted.unwrap(), Ok(0));
                assert_eq!(encoded.unwrap(), Ok(()));
            }
        }
        assert_eq!(encode_length::<Fixed<2>>(2), Ok(vec![]));
        assert_eq!(decode_length::<Fixed<2>>(b""), Ok(Some(2)));
        assert_eq!(decode_length::<Rest>(b""), Ok(None));

        type Exact = Field<Ascii<2, 2>, Fixed<2>>;
        assert_eq!(Exact::encoded_len(b"ABC"), Err(Error::InvalidValueLength));
        assert_eq!(
            Exact::encode(&mut &mut [0; 8][..], &mut [][..], b"ABC"),
            Err(Error::InvalidValueLength)
        );
    }
}
