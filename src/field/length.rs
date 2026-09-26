use core::marker::PhantomData;

use super::{Identity, Step};
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

#[inline(always)]
fn declared_wire_len<S: Step>(count: usize) -> Result<usize, Error> {
    S::encoded_len_of_count(count).map_err(|error| {
        cold_path();
        if error == Error::BufferOverflow { Error::Invalid } else { error }
    })
}

/// Framing information after decoding any length prefix.
pub struct DecodePlan {
    /// Number of payload bytes to take from the input.
    pub wire_len: usize,
    /// The field's width in value units, when the framing carries it: from a
    /// prefix that counts value units, or from `Fixed<N>`.
    pub count: Option<usize>,
}

/// How a field states its extent. `count` is the field's width in value
/// units, the units the step chain starts from, including width padding;
/// `wire_len` is the step chain's output in bytes. Each spec frames with one
/// of them.
/// What a length spec states about the value it frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Framing {
    /// A count of value units, known before the value: `Fixed` and the counting
    /// prefixes. A list's value units are its items.
    Count,
    /// The value's length in bytes, written before the value: the wire-length
    /// prefixes.
    Bytes,
    /// Everything the enclosing format supplies: `Rest`.
    Rest,
}

pub trait LengthSpec<S: Step> {
    /// What the framing states, so a list knows whether it must measure its
    /// items before writing the prefix.
    const FRAMING: Framing;

    /// Encoded prefix size, excluding the payload, after checking length limits.
    fn encoded_len(count: usize, wire_len: usize) -> Result<usize, Error>;
    /// Write the prefix, if any.
    fn encode(output: &mut &mut [u8], scratch: &mut [u8], count: usize, wire_len: usize) -> Result<(), Error>;
    /// Read the prefix, if any, and say how many payload bytes follow.
    fn decode_plan<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<DecodePlan, Error>;
}

/// A field `N` value units wide, however it is encoded: `Fixed<19>` holds 19
/// digits, packed or not. Shorter values need width padding to reach `N`;
/// decoding strips that padding down to the padding step's minimum length.
/// Encoding checks in debug builds that the value fills the width.
pub struct Fixed<const N: usize>;

impl<const N: usize, S: Step> LengthSpec<S> for Fixed<N> {
    const FRAMING: Framing = Framing::Count;

    #[inline(always)]
    fn encoded_len(count: usize, wire_len: usize) -> Result<usize, Error> {
        debug_assert_eq!(count, N, "value does not fill the fixed width");
        debug_assert_eq!(S::encoded_len_of_count(N), Ok(wire_len));
        Ok(0)
    }

    #[inline(always)]
    fn encode(_output: &mut &mut [u8], _scratch: &mut [u8], count: usize, wire_len: usize) -> Result<(), Error> {
        <Self as LengthSpec<S>>::encoded_len(count, wire_len)?;
        Ok(())
    }

    #[inline(always)]
    fn decode_plan<'a>(_input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<DecodePlan, Error> {
        let wire_len = S::encoded_len_of_count(N)?;
        Ok(DecodePlan { wire_len, count: Some(N) })
    }
}

/// A prefix in format `F` counting the value units that follow, including
/// width padding. Steps convert the count to bytes.
pub struct Length<F>(PhantomData<F>);

impl<F: ScalarFmt, S: Step> LengthSpec<S> for Length<F> {
    const FRAMING: Framing = Framing::Count;

    #[inline(always)]
    fn encoded_len(count: usize, _wire_len: usize) -> Result<usize, Error> {
        F::encoded_len_usize(count)
    }

    #[inline(always)]
    fn encode(output: &mut &mut [u8], scratch: &mut [u8], count: usize, _wire_len: usize) -> Result<(), Error> {
        F::encode_usize(output, scratch, count)
    }

    #[inline(always)]
    fn decode_plan<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<DecodePlan, Error> {
        let count = F::decode_usize(input, scratch)?;
        let wire_len = declared_wire_len::<S>(count)?;
        Ok(DecodePlan {
            wire_len,
            count: Some(count),
        })
    }
}

/// A prefix in format `F` counting the bytes that follow.
pub struct WireLength<F>(PhantomData<F>);

impl<F: ScalarFmt, S: Step> LengthSpec<S> for WireLength<F> {
    const FRAMING: Framing = Framing::Bytes;

    #[inline(always)]
    fn encoded_len(_count: usize, wire_len: usize) -> Result<usize, Error> {
        F::encoded_len_usize(wire_len)
    }

    #[inline(always)]
    fn encode(output: &mut &mut [u8], scratch: &mut [u8], _count: usize, wire_len: usize) -> Result<(), Error> {
        F::encode_usize(output, scratch, wire_len)
    }

    #[inline(always)]
    fn decode_plan<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<DecodePlan, Error> {
        let wire_len = F::decode_usize(input, scratch)?;
        Ok(DecodePlan { wire_len, count: None })
    }
}

/// `N` ASCII decimal digits counting the value units that follow, as in
/// [`Length`].
pub struct AsciiLength<const N: usize>;

impl<const N: usize, S: Step> LengthSpec<S> for AsciiLength<N> {
    const FRAMING: Framing = Framing::Count;

    #[inline(always)]
    fn encoded_len(count: usize, _wire_len: usize) -> Result<usize, Error> {
        decimal_prefix_len(count, N)
    }

    #[inline(always)]
    fn encode(output: &mut &mut [u8], _scratch: &mut [u8], count: usize, _wire_len: usize) -> Result<(), Error> {
        encode_ascii_decimal_fixed(output, count, N)
    }

    #[inline(always)]
    fn decode_plan<'a>(input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<DecodePlan, Error> {
        let count = decode_ascii_decimal_fixed(input, N)?;
        let wire_len = declared_wire_len::<S>(count)?;
        Ok(DecodePlan {
            wire_len,
            count: Some(count),
        })
    }
}

/// `N` ASCII decimal digits counting the bytes that follow.
pub struct AsciiWireLength<const N: usize>;

impl<const N: usize, S: Step> LengthSpec<S> for AsciiWireLength<N> {
    const FRAMING: Framing = Framing::Bytes;

    #[inline(always)]
    fn encoded_len(_count: usize, wire_len: usize) -> Result<usize, Error> {
        decimal_prefix_len(wire_len, N)
    }

    #[inline(always)]
    fn encode(output: &mut &mut [u8], _scratch: &mut [u8], _count: usize, wire_len: usize) -> Result<(), Error> {
        encode_ascii_decimal_fixed(output, wire_len, N)
    }

    #[inline(always)]
    fn decode_plan<'a>(input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<DecodePlan, Error> {
        let wire_len = decode_ascii_decimal_fixed(input, N)?;
        Ok(DecodePlan { wire_len, count: None })
    }
}

/// `N` EBCDIC decimal digits counting the value units that follow, as in
/// [`Length`].
pub struct EbcdicLength<const N: usize>;

impl<const N: usize, S: Step> LengthSpec<S> for EbcdicLength<N> {
    const FRAMING: Framing = Framing::Count;

    #[inline(always)]
    fn encoded_len(count: usize, _wire_len: usize) -> Result<usize, Error> {
        decimal_prefix_len(count, N)
    }

    #[inline(always)]
    fn encode(output: &mut &mut [u8], _scratch: &mut [u8], count: usize, _wire_len: usize) -> Result<(), Error> {
        encode_ebcdic_decimal_fixed(output, count, N)
    }

    #[inline(always)]
    fn decode_plan<'a>(input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<DecodePlan, Error> {
        let count = decode_ebcdic_decimal_fixed(input, N)?;
        let wire_len = declared_wire_len::<S>(count)?;
        Ok(DecodePlan {
            wire_len,
            count: Some(count),
        })
    }
}

/// [`EbcdicLength`] that encodes zero as `N` EBCDIC blanks and decodes
/// either blanks or zero digits as zero.
pub struct BlankableEbcdicLength<const N: usize>;

impl<const N: usize, S: Step> LengthSpec<S> for BlankableEbcdicLength<N> {
    const FRAMING: Framing = Framing::Count;

    #[inline(always)]
    fn encoded_len(count: usize, _wire_len: usize) -> Result<usize, Error> {
        decimal_prefix_len(count, N)
    }

    #[inline(always)]
    fn encode(output: &mut &mut [u8], _scratch: &mut [u8], count: usize, _wire_len: usize) -> Result<(), Error> {
        encode_ebcdic_decimal_blank_zero_fixed(output, count, N)
    }

    #[inline(always)]
    fn decode_plan<'a>(input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<DecodePlan, Error> {
        let count = decode_ebcdic_decimal_blank_zero_fixed(input, N)?;
        let wire_len = declared_wire_len::<S>(count)?;
        Ok(DecodePlan {
            wire_len,
            count: Some(count),
        })
    }
}

/// `N` EBCDIC decimal digits counting the bytes that follow.
pub struct EbcdicWireLength<const N: usize>;

impl<const N: usize, S: Step> LengthSpec<S> for EbcdicWireLength<N> {
    const FRAMING: Framing = Framing::Bytes;

    #[inline(always)]
    fn encoded_len(_count: usize, wire_len: usize) -> Result<usize, Error> {
        decimal_prefix_len(wire_len, N)
    }

    #[inline(always)]
    fn encode(output: &mut &mut [u8], _scratch: &mut [u8], _count: usize, wire_len: usize) -> Result<(), Error> {
        encode_ebcdic_decimal_fixed(output, wire_len, N)
    }

    #[inline(always)]
    fn decode_plan<'a>(input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<DecodePlan, Error> {
        let wire_len = decode_ebcdic_decimal_fixed(input, N)?;
        Ok(DecodePlan { wire_len, count: None })
    }
}

/// A length prefix `L` whose value is the length plus `K`, for lengths that also
/// count bytes or items outside the payload: the prefix itself, a header before
/// it, or a header item. An IBM RDW is `Offset<WireLength<FixedBinaryBe<2>>, 2>`
/// framing its two reserved bytes and the record. Only for [`Identity`]
/// framing, where the length's units need no conversion. A value below `K` is
/// `Invalid`.
pub struct Offset<L, const K: usize>(PhantomData<L>);

#[inline(always)]
fn add_offset<const K: usize>(len: usize) -> Result<usize, Error> {
    len.checked_add(K).ok_or_else(|| {
        cold_path();
        Error::Invalid
    })
}

#[inline(always)]
fn remove_offset<const K: usize>(len: usize) -> Result<usize, Error> {
    len.checked_sub(K).ok_or_else(|| {
        cold_path();
        Error::Invalid
    })
}

impl<L: LengthSpec<Identity>, const K: usize> LengthSpec<Identity> for Offset<L, K> {
    const FRAMING: Framing = L::FRAMING;

    #[inline(always)]
    fn encoded_len(count: usize, wire_len: usize) -> Result<usize, Error> {
        L::encoded_len(add_offset::<K>(count)?, add_offset::<K>(wire_len)?)
    }

    #[inline(always)]
    fn encode(output: &mut &mut [u8], scratch: &mut [u8], count: usize, wire_len: usize) -> Result<(), Error> {
        L::encode(output, scratch, add_offset::<K>(count)?, add_offset::<K>(wire_len)?)
    }

    #[inline(always)]
    fn decode_plan<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<DecodePlan, Error> {
        let plan = L::decode_plan(input, scratch)?;
        Ok(DecodePlan {
            wire_len: remove_offset::<K>(plan.wire_len)?,
            count: plan.count.map(remove_offset::<K>).transpose()?,
        })
    }
}

/// Consume the entire remaining input supplied by the enclosing format.
/// A following sibling requires an enclosing frame that bounds this input.
pub struct Rest;

impl<S: Step> LengthSpec<S> for Rest {
    const FRAMING: Framing = Framing::Rest;

    #[inline(always)]
    fn encoded_len(_count: usize, _wire_len: usize) -> Result<usize, Error> {
        Ok(0)
    }

    #[inline(always)]
    fn encode(_output: &mut &mut [u8], _scratch: &mut [u8], _count: usize, _wire_len: usize) -> Result<(), Error> {
        Ok(())
    }

    #[inline(always)]
    fn decode_plan<'a>(input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<DecodePlan, Error> {
        let wire_len = input.len();
        Ok(DecodePlan { wire_len, count: None })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AsciiLength, AsciiWireLength, BlankableEbcdicLength, EbcdicLength, EbcdicWireLength, Fixed, Length, LengthSpec, Offset, WireLength,
        encode_ascii_decimal_fixed,
    };
    use crate::field::{Ascii, Field, FixedBinaryBe, Identity, Numeric, PadLeft, PadRightEven, Step, UnpackNibbles};
    use crate::{Error, ScalarFmt};

    fn encode_length<L: LengthSpec<Identity>>(count: usize, wire_len: usize) -> Result<Vec<u8>, Error> {
        let mut output = [0; 32];
        let mut out = output.as_mut_slice();
        let encoded = L::encode(&mut out, &mut [][..], count, wire_len);
        let written = 32 - out.len();
        assert_eq!(L::encoded_len(count, wire_len), encoded.map(|()| written));
        encoded.map(|()| output[..written].to_vec())
    }

    fn decode_semantic<L: LengthSpec<Identity>>(input: &[u8]) -> (usize, Option<usize>) {
        let mut input = input;
        let mut scratch = [];
        let mut scratch_ptr = scratch.as_mut_slice();
        let plan = L::decode_plan(&mut input, &mut scratch_ptr).unwrap();
        (plan.wire_len, plan.count)
    }

    #[test]
    fn offset_prefixes_count_what_lies_outside_the_payload() {
        // An IBM RDW: a 2-byte length counting itself and two reserved bytes.
        type Rdw = Offset<WireLength<FixedBinaryBe<2>>, 2>;
        assert_eq!(encode_length::<Rdw>(0, 10).unwrap(), [0x00, 0x0C]);
        assert_eq!(decode_semantic::<Rdw>(&[0x00, 0x0C]), (10, None));
        // A count that includes a header item.
        type Items = Offset<AsciiLength<5>, 1>;
        assert_eq!(encode_length::<Items>(2, 2).unwrap(), b"00003");
        assert_eq!(decode_semantic::<Items>(b"00003"), (2, Some(2)));
        for wire in [&[0x00, 0x01][..], &[0x00, 0x00]] {
            assert!(Rdw::decode_plan(&mut &wire[..], &mut &mut [][..]).is_err_and(|error| error == Error::Invalid));
        }
        assert_eq!(
            Items::decode_plan(&mut &b"00000"[..], &mut &mut [][..]).map(|plan| plan.count),
            Err(Error::Invalid)
        );

        type Record = Field<crate::Binary<0, 20>, Rdw>;
        let mut output = [0; 8];
        Record::encode(&mut &mut output[..], &mut [], b"\0\0AB").unwrap();
        assert_eq!(output[..6], *b"\x00\x06\0\0AB");
        assert_eq!(Record::decode(&mut &output[..6], &mut &mut [][..]), Ok(&b"\0\0AB"[..]));
    }

    #[test]
    fn test_ascii_length_specs() {
        assert_eq!(encode_length::<AsciiLength<2>>(16, 0).unwrap(), b"16");
        assert_eq!(encode_length::<AsciiLength<3>>(255, 0).unwrap(), b"255");
        assert_eq!(encode_length::<AsciiLength<4>>(9999, 0).unwrap(), b"9999");
        assert_eq!(decode_semantic::<AsciiLength<2>>(b"16"), (16, Some(16)));
        assert_eq!(decode_semantic::<AsciiWireLength<2>>(b"19"), (19, None));
    }

    #[test]
    fn test_ebcdic_length_specs() {
        assert_eq!(encode_length::<EbcdicLength<2>>(16, 0).unwrap(), [0xF1, 0xF6]);
        assert_eq!(encode_length::<EbcdicLength<3>>(255, 0).unwrap(), [0xF2, 0xF5, 0xF5]);
        assert_eq!(encode_length::<EbcdicLength<4>>(9999, 0).unwrap(), [0xF9, 0xF9, 0xF9, 0xF9]);
        assert_eq!(decode_semantic::<EbcdicLength<2>>(&[0xF1, 0xF6]), (16, Some(16)));
        assert_eq!(decode_semantic::<EbcdicWireLength<2>>(&[0xF1, 0xF9]), (19, None));
        assert_eq!(encode_length::<BlankableEbcdicLength<2>>(0, 0).unwrap(), [0x40, 0x40]);
        assert_eq!(encode_length::<BlankableEbcdicLength<2>>(16, 0).unwrap(), [0xF1, 0xF6]);
        assert_eq!(decode_semantic::<BlankableEbcdicLength<2>>(&[0x40, 0x40]), (0, Some(0)));
        assert_eq!(decode_semantic::<BlankableEbcdicLength<2>>(&[0xF1, 0xF6]), (16, Some(16)));
    }

    #[test]
    fn test_length_overflow_and_invalid() {
        let mut output = [0u8; 2];
        let mut out = output.as_mut_slice();
        let mut scratch = [0u8; 0];
        assert!(<AsciiLength<2> as LengthSpec<Identity>>::encode(&mut out, scratch.as_mut_slice(), 100, 0).is_err());

        let mut input = &b"A0"[..];
        let mut scratch = [];
        let mut scratch_ptr = scratch.as_mut_slice();
        assert!(<AsciiLength<2> as LengthSpec<Identity>>::decode_plan(&mut input, &mut scratch_ptr).is_err());

        let mut input = &[0xF0, b'0'][..];
        let mut scratch = [];
        let mut scratch_ptr = scratch.as_mut_slice();
        assert!(<EbcdicLength<2> as LengthSpec<Identity>>::decode_plan(&mut input, &mut scratch_ptr).is_err());

        let mut input = &[0x40, 0xF0][..];
        let mut scratch = [];
        let mut scratch_ptr = scratch.as_mut_slice();
        assert!(<BlankableEbcdicLength<2> as LengthSpec<Identity>>::decode_plan(&mut input, &mut scratch_ptr).is_err());
    }

    #[test]
    fn fixed_framing_asserts_only_in_debug() {
        fn check<L: LengthSpec<Identity>>() {
            // A short count with the right wire length is a packed value one digit short.
            for (count, wire_len) in [(0, 0), (1, 1), (3, 3), (1, 2)] {
                let predicted = std::panic::catch_unwind(|| L::encoded_len(count, wire_len));
                let encoded = std::panic::catch_unwind(|| L::encode(&mut &mut [][..], &mut [][..], count, wire_len));
                if cfg!(debug_assertions) {
                    assert!(predicted.is_err());
                    assert!(encoded.is_err());
                } else {
                    assert_eq!(predicted.unwrap(), Ok(0));
                    assert_eq!(encoded.unwrap(), Ok(()));
                }
            }
            assert_eq!(L::encoded_len(2, 2), Ok(0));
        }
        check::<Fixed<2>>();
    }

    #[test]
    fn fixed_framing_preserves_input_validation_and_padding() {
        type Exact = Field<Ascii<2, 2>, Fixed<2>>;
        assert_eq!(Exact::encoded_len(b"ABC"), Err(Error::InvalidValueLength));
        assert_eq!(
            Exact::encode(&mut &mut [0; 8][..], &mut [][..], b"ABC"),
            Err(Error::InvalidValueLength)
        );
        type Padded = Field<Numeric<1, 4>, Fixed<4>, PadLeft<4, b'0'>>;
        let mut output = [0; 4];
        assert_eq!(Padded::encoded_len(b"7"), Ok(4));
        Padded::encode(&mut &mut output[..], &mut [][..], b"7").unwrap();
        assert_eq!(output, *b"0007");
    }
    #[test]
    fn decimal_prefixes_validate_the_selected_length() {
        for value in [0, 1, 9, 10, 99, 100, usize::MAX] {
            let expected = if value < 100 { Ok(2) } else { Err(Error::Invalid) };
            assert_eq!(encode_length::<AsciiLength<2>>(value, usize::MAX).map(|v| v.len()), expected);
            assert_eq!(encode_length::<AsciiWireLength<2>>(usize::MAX, value).map(|v| v.len()), expected);
            assert_eq!(encode_length::<EbcdicLength<2>>(value, usize::MAX).map(|v| v.len()), expected);
            assert_eq!(encode_length::<EbcdicWireLength<2>>(usize::MAX, value).map(|v| v.len()), expected);
            assert_eq!(
                encode_length::<BlankableEbcdicLength<2>>(value, usize::MAX).map(|v| v.len()),
                expected
            );
        }
        assert_eq!(encode_length::<AsciiLength<20>>(usize::MAX, 0).map(|v| v.len()), Ok(20));
        assert_eq!(encode_length::<AsciiLength<32>>(usize::MAX, 0).map(|v| v.len()), Ok(32));
    }
    #[test]
    fn impossible_declared_lengths_are_invalid() {
        fn decode<S: Step, L: LengthSpec<S>>(wire: &[u8]) -> Result<(), Error> {
            L::decode_plan(&mut &wire[..], &mut &mut [][..]).map(|_| ())
        }
        let max = (usize::MAX as u64).to_be_bytes();
        assert_eq!(decode::<PadRightEven, Length<FixedBinaryBe<8>>>(&max), Err(Error::Invalid));
        assert_eq!(
            decode::<UnpackNibbles<crate::primitive::nibble::UpperHexDigits>, Length<FixedBinaryBe<8>>>(&max),
            Err(Error::Invalid)
        );
        let mut ascii = [0; 20];
        encode_ascii_decimal_fixed(&mut &mut ascii[..], usize::MAX, 20).unwrap();
        let ebcdic = ascii.map(|b| b - b'0' + 0xF0);
        assert_eq!(decode::<PadRightEven, AsciiLength<20>>(&ascii), Err(Error::Invalid));
        assert_eq!(decode::<PadRightEven, EbcdicLength<20>>(&ebcdic), Err(Error::Invalid));
        assert_eq!(decode::<PadRightEven, BlankableEbcdicLength<20>>(&ebcdic), Err(Error::Invalid));
    }
}
