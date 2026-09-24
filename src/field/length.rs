use core::marker::PhantomData;

use super::Step;
use crate::primitive::decimal::{
    MAX_INTEGER_TEXT_LEN, decode_decimal_ascii_fixed, decode_decimal_ebcdic_blank_zero_fixed, decode_decimal_ebcdic_fixed,
    encode_decimal_ascii_fixed, encode_decimal_ebcdic_blank_zero_fixed, encode_decimal_ebcdic_fixed, format_u64,
};
use crate::utils::cold_path;
use crate::{Error, ScalarFmt};

#[inline(always)]
fn decimal_prefix_len(value: usize, width: usize) -> Result<usize, Error> {
    let mut digits = [0; MAX_INTEGER_TEXT_LEN];
    if format_u64(&mut digits, value as u64).len() > width {
        cold_path();
        return Err(Error::Invalid);
    }
    Ok(width)
}

#[inline(always)]
fn declared_wire_len<S: Step>(semantic_len: usize) -> Result<usize, Error> {
    S::encoded_len(semantic_len).map_err(|error| {
        cold_path();
        if error == Error::BufferOverflow { Error::Invalid } else { error }
    })
}

/// Framing information after decoding any length prefix.
pub struct DecodePlan {
    /// Number of payload bytes to take from the input.
    pub wire_len: usize,
    /// Exact decoded logical length, when carried by the framing.
    pub semantic_len: Option<usize>,
}

pub trait LengthSpec<S: Step> {
    /// Encoded prefix size, excluding the payload, after checking length limits.
    fn encoded_len(semantic_len: usize, wire_len: usize) -> Result<usize, Error>;
    fn encode(output: &mut &mut [u8], scratch: &mut [u8], semantic_len: usize, wire_len: usize) -> Result<(), Error>;
    fn decode_plan<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<DecodePlan, Error>;
}

/// Fix the decoded logical length to `N` and frame `S::encoded_len(N)` bytes.
/// The check and transform must agree with that width, including any explicitly
/// selected padding or truncation. Encoding checks this invariant in debug builds.
pub struct Fixed<const N: usize>;

impl<const N: usize, S: Step> LengthSpec<S> for Fixed<N> {
    #[inline(always)]
    fn encoded_len(_semantic_len: usize, wire_len: usize) -> Result<usize, Error> {
        debug_assert_eq!(S::encoded_len(N), Ok(wire_len));
        Ok(0)
    }

    #[inline(always)]
    fn encode(_output: &mut &mut [u8], _scratch: &mut [u8], semantic_len: usize, wire_len: usize) -> Result<(), Error> {
        <Self as LengthSpec<S>>::encoded_len(semantic_len, wire_len)?;
        Ok(())
    }

    #[inline(always)]
    fn decode_plan<'a>(_input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<DecodePlan, Error> {
        let wire_len = S::encoded_len(N)?;
        Ok(DecodePlan {
            wire_len,
            semantic_len: Some(N),
        })
    }
}

/// Frame exactly `N` wire bytes; the step determines the decoded logical length.
/// The check and transform must produce this width; encoding asserts it in debug.
pub struct WireFixed<const N: usize>;

impl<const N: usize, S: Step> LengthSpec<S> for WireFixed<N> {
    #[inline(always)]
    fn encoded_len(_semantic_len: usize, wire_len: usize) -> Result<usize, Error> {
        debug_assert_eq!(wire_len, N);
        Ok(0)
    }

    #[inline(always)]
    fn encode(_output: &mut &mut [u8], _scratch: &mut [u8], semantic_len: usize, wire_len: usize) -> Result<(), Error> {
        <Self as LengthSpec<S>>::encoded_len(semantic_len, wire_len)?;
        Ok(())
    }

    #[inline(always)]
    fn decode_plan<'a>(_input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<DecodePlan, Error> {
        Ok(DecodePlan {
            wire_len: N,
            semantic_len: None,
        })
    }
}

pub struct Length<F>(PhantomData<F>);

impl<F: ScalarFmt, S: Step> LengthSpec<S> for Length<F> {
    #[inline(always)]
    fn encoded_len(semantic_len: usize, _wire_len: usize) -> Result<usize, Error> {
        F::encoded_len_usize(semantic_len)
    }

    #[inline(always)]
    fn encode(output: &mut &mut [u8], scratch: &mut [u8], semantic_len: usize, _wire_len: usize) -> Result<(), Error> {
        F::encode_usize(output, scratch, semantic_len)
    }

    #[inline(always)]
    fn decode_plan<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<DecodePlan, Error> {
        let semantic_len = F::decode_usize(input, scratch)?;
        let wire_len = declared_wire_len::<S>(semantic_len)?;
        Ok(DecodePlan {
            wire_len,
            semantic_len: Some(semantic_len),
        })
    }
}

pub struct WireLength<F>(PhantomData<F>);

impl<F: ScalarFmt, S: Step> LengthSpec<S> for WireLength<F> {
    #[inline(always)]
    fn encoded_len(_semantic_len: usize, wire_len: usize) -> Result<usize, Error> {
        F::encoded_len_usize(wire_len)
    }

    #[inline(always)]
    fn encode(output: &mut &mut [u8], scratch: &mut [u8], _semantic_len: usize, wire_len: usize) -> Result<(), Error> {
        F::encode_usize(output, scratch, wire_len)
    }

    #[inline(always)]
    fn decode_plan<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<DecodePlan, Error> {
        let wire_len = F::decode_usize(input, scratch)?;
        Ok(DecodePlan {
            wire_len,
            semantic_len: None,
        })
    }
}

pub struct AsciiLength<const N: usize>;

impl<const N: usize, S: Step> LengthSpec<S> for AsciiLength<N> {
    #[inline(always)]
    fn encoded_len(semantic_len: usize, _wire_len: usize) -> Result<usize, Error> {
        decimal_prefix_len(semantic_len, N)
    }

    #[inline(always)]
    fn encode(output: &mut &mut [u8], _scratch: &mut [u8], semantic_len: usize, _wire_len: usize) -> Result<(), Error> {
        encode_decimal_ascii_fixed(output, semantic_len, N)
    }

    #[inline(always)]
    fn decode_plan<'a>(input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<DecodePlan, Error> {
        let semantic_len = decode_decimal_ascii_fixed(input, N)?;
        let wire_len = declared_wire_len::<S>(semantic_len)?;
        Ok(DecodePlan {
            wire_len,
            semantic_len: Some(semantic_len),
        })
    }
}

pub struct AsciiWireLength<const N: usize>;

impl<const N: usize, S: Step> LengthSpec<S> for AsciiWireLength<N> {
    #[inline(always)]
    fn encoded_len(_semantic_len: usize, wire_len: usize) -> Result<usize, Error> {
        decimal_prefix_len(wire_len, N)
    }

    #[inline(always)]
    fn encode(output: &mut &mut [u8], _scratch: &mut [u8], _semantic_len: usize, wire_len: usize) -> Result<(), Error> {
        encode_decimal_ascii_fixed(output, wire_len, N)
    }

    #[inline(always)]
    fn decode_plan<'a>(input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<DecodePlan, Error> {
        let wire_len = decode_decimal_ascii_fixed(input, N)?;
        Ok(DecodePlan {
            wire_len,
            semantic_len: None,
        })
    }
}

pub struct EbcdicLength<const N: usize>;

impl<const N: usize, S: Step> LengthSpec<S> for EbcdicLength<N> {
    #[inline(always)]
    fn encoded_len(semantic_len: usize, _wire_len: usize) -> Result<usize, Error> {
        decimal_prefix_len(semantic_len, N)
    }

    #[inline(always)]
    fn encode(output: &mut &mut [u8], _scratch: &mut [u8], semantic_len: usize, _wire_len: usize) -> Result<(), Error> {
        encode_decimal_ebcdic_fixed(output, semantic_len, N)
    }

    #[inline(always)]
    fn decode_plan<'a>(input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<DecodePlan, Error> {
        let semantic_len = decode_decimal_ebcdic_fixed(input, N)?;
        let wire_len = declared_wire_len::<S>(semantic_len)?;
        Ok(DecodePlan {
            wire_len,
            semantic_len: Some(semantic_len),
        })
    }
}

pub struct BlankableEbcdicLength<const N: usize>;

impl<const N: usize, S: Step> LengthSpec<S> for BlankableEbcdicLength<N> {
    #[inline(always)]
    fn encoded_len(semantic_len: usize, _wire_len: usize) -> Result<usize, Error> {
        if semantic_len == 0 {
            Ok(N)
        } else {
            decimal_prefix_len(semantic_len, N)
        }
    }

    #[inline(always)]
    fn encode(output: &mut &mut [u8], _scratch: &mut [u8], semantic_len: usize, _wire_len: usize) -> Result<(), Error> {
        encode_decimal_ebcdic_blank_zero_fixed(output, semantic_len, N)
    }

    #[inline(always)]
    fn decode_plan<'a>(input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<DecodePlan, Error> {
        let semantic_len = decode_decimal_ebcdic_blank_zero_fixed(input, N)?;
        let wire_len = declared_wire_len::<S>(semantic_len)?;
        Ok(DecodePlan {
            wire_len,
            semantic_len: Some(semantic_len),
        })
    }
}

pub struct EbcdicWireLength<const N: usize>;

impl<const N: usize, S: Step> LengthSpec<S> for EbcdicWireLength<N> {
    #[inline(always)]
    fn encoded_len(_semantic_len: usize, wire_len: usize) -> Result<usize, Error> {
        decimal_prefix_len(wire_len, N)
    }

    #[inline(always)]
    fn encode(output: &mut &mut [u8], _scratch: &mut [u8], _semantic_len: usize, wire_len: usize) -> Result<(), Error> {
        encode_decimal_ebcdic_fixed(output, wire_len, N)
    }

    #[inline(always)]
    fn decode_plan<'a>(input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<DecodePlan, Error> {
        let wire_len = decode_decimal_ebcdic_fixed(input, N)?;
        Ok(DecodePlan {
            wire_len,
            semantic_len: None,
        })
    }
}

/// Consume the entire remaining input supplied by the enclosing format.
/// A following sibling requires an enclosing frame that bounds this input.
pub struct Rest;

impl<S: Step> LengthSpec<S> for Rest {
    #[inline(always)]
    fn encoded_len(_semantic_len: usize, _wire_len: usize) -> Result<usize, Error> {
        Ok(0)
    }

    #[inline(always)]
    fn encode(_output: &mut &mut [u8], _scratch: &mut [u8], _semantic_len: usize, _wire_len: usize) -> Result<(), Error> {
        Ok(())
    }

    #[inline(always)]
    fn decode_plan<'a>(input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<DecodePlan, Error> {
        let wire_len = input.len();
        Ok(DecodePlan {
            wire_len,
            semantic_len: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AsciiLength, AsciiWireLength, BlankableEbcdicLength, EbcdicLength, EbcdicWireLength, Fixed, Length, LengthSpec, WireFixed,
        encode_decimal_ascii_fixed,
    };
    use crate::field::{Ascii, Binary, Field, FixedBinaryBe, Identity, Numeric, PadLeft, PadRightEven, Step, UnpackNibbles};
    use crate::{Error, ScalarFmt};

    fn encode_length<L: LengthSpec<Identity>>(semantic_len: usize, wire_len: usize) -> Result<Vec<u8>, Error> {
        let mut output = [0; 32];
        let mut out = output.as_mut_slice();
        let encoded = L::encode(&mut out, &mut [][..], semantic_len, wire_len);
        let written = 32 - out.len();
        assert_eq!(L::encoded_len(semantic_len, wire_len), encoded.map(|()| written));
        encoded.map(|()| output[..written].to_vec())
    }

    fn decode_semantic<L: LengthSpec<Identity>>(input: &[u8]) -> (usize, Option<usize>) {
        let mut input = input;
        let mut scratch = [];
        let mut scratch_ptr = scratch.as_mut_slice();
        let plan = L::decode_plan(&mut input, &mut scratch_ptr).unwrap();
        (plan.wire_len, plan.semantic_len)
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
    fn test_wire_fixed_length_spec() {
        let mut input = &b"ignored"[..];
        let mut scratch = [];
        let mut scratch_ptr = scratch.as_mut_slice();
        let plan = <WireFixed<7> as LengthSpec<Identity>>::decode_plan(&mut input, &mut scratch_ptr).unwrap();
        assert_eq!(plan.wire_len, 7);
        assert_eq!(plan.semantic_len, None);
    }

    #[test]
    fn fixed_framing_asserts_only_in_debug() {
        fn check<L: LengthSpec<Identity>>() {
            for len in [0, 1, 3] {
                let predicted = std::panic::catch_unwind(|| L::encoded_len(len, len));
                let encoded = std::panic::catch_unwind(|| L::encode(&mut &mut [][..], &mut [][..], len, len));
                if cfg!(debug_assertions) {
                    assert!(predicted.is_err());
                    assert!(encoded.is_err());
                } else {
                    assert_eq!(predicted.unwrap(), Ok(0));
                    assert_eq!(encoded.unwrap(), Ok(()));
                }
            }
            // Framing does not forbid an explicit transform from shortening input.
            assert_eq!(L::encoded_len(usize::MAX, 2), Ok(0));
        }
        check::<Fixed<2>>();
        check::<WireFixed<2>>();
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
        assert_eq!(
            Field::<Binary<0, 100>, AsciiLength<1>>::encoded_len(b"0123456789"),
            Err(Error::InvalidValueLength)
        );
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
        encode_decimal_ascii_fixed(&mut &mut ascii[..], usize::MAX, 20).unwrap();
        let ebcdic = ascii.map(|b| b - b'0' + 0xF0);
        assert_eq!(decode::<PadRightEven, AsciiLength<20>>(&ascii), Err(Error::Invalid));
        assert_eq!(decode::<PadRightEven, EbcdicLength<20>>(&ebcdic), Err(Error::Invalid));
        assert_eq!(decode::<PadRightEven, BlankableEbcdicLength<20>>(&ebcdic), Err(Error::Invalid));
    }
}

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[test]
        fn prefix_prediction_matches_encoding(value in any::<usize>(), width in 1usize..33) {
            let mut output = [0; 32];
            let mut out = &mut output[..];
            let encoded = encode_decimal_ascii_fixed(&mut out, value, width).map(|()| 32 - out.len());
            prop_assert_eq!(decimal_prefix_len(value, width), encoded);
        }
    }
}
