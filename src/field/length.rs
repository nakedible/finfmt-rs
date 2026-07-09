use core::marker::PhantomData;

use super::Step;
use crate::primitive::decimal::{
    decode_decimal_ascii_fixed, decode_decimal_ebcdic_blankable_fixed, decode_decimal_ebcdic_fixed, encode_decimal_ascii_fixed,
    encode_decimal_ebcdic_blankable_fixed, encode_decimal_ebcdic_fixed,
};
use crate::{Error, ScalarFmt};

pub struct DecodePlan {
    pub output_cap: usize,
    pub wire_len: usize,
    pub exact_len: Option<usize>,
}

pub trait LengthSpec<S: Step> {
    fn encoded_len(semantic_len: usize, wire_len: usize) -> Result<usize, Error>;
    fn encode(output: &mut &mut [u8], scratch: &mut &mut [u8], semantic_len: usize, wire_len: usize) -> Result<(), Error>;
    fn decode_plan<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<DecodePlan, Error>;
}

pub struct Fixed<const N: usize>;

impl<const N: usize, S: Step> LengthSpec<S> for Fixed<N> {
    #[inline(always)]
    fn encoded_len(_semantic_len: usize, _wire_len: usize) -> Result<usize, Error> {
        Ok(0)
    }

    #[inline(always)]
    fn encode(_output: &mut &mut [u8], _scratch: &mut &mut [u8], _semantic_len: usize, _wire_len: usize) -> Result<(), Error> {
        Ok(())
    }

    #[inline(always)]
    fn decode_plan<'a>(_input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<DecodePlan, Error> {
        let wire_len = S::encoded_len(N)?;
        Ok(DecodePlan {
            output_cap: S::decoded_max_len(wire_len)?,
            wire_len,
            exact_len: Some(N),
        })
    }
}

pub struct WireFixed<const N: usize>;

impl<const N: usize, S: Step> LengthSpec<S> for WireFixed<N> {
    #[inline(always)]
    fn encoded_len(_semantic_len: usize, _wire_len: usize) -> Result<usize, Error> {
        Ok(0)
    }

    #[inline(always)]
    fn encode(_output: &mut &mut [u8], _scratch: &mut &mut [u8], _semantic_len: usize, _wire_len: usize) -> Result<(), Error> {
        Ok(())
    }

    #[inline(always)]
    fn decode_plan<'a>(_input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<DecodePlan, Error> {
        Ok(DecodePlan {
            output_cap: S::decoded_max_len(N)?,
            wire_len: N,
            exact_len: None,
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
    fn encode(output: &mut &mut [u8], scratch: &mut &mut [u8], semantic_len: usize, _wire_len: usize) -> Result<(), Error> {
        F::encode_usize(output, scratch, semantic_len)
    }

    #[inline(always)]
    fn decode_plan<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<DecodePlan, Error> {
        let semantic_len = F::decode_usize(input, scratch)?;
        let wire_len = S::encoded_len(semantic_len)?;
        Ok(DecodePlan {
            output_cap: S::decoded_max_len(wire_len)?,
            wire_len,
            exact_len: Some(semantic_len),
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
    fn encode(output: &mut &mut [u8], scratch: &mut &mut [u8], _semantic_len: usize, wire_len: usize) -> Result<(), Error> {
        F::encode_usize(output, scratch, wire_len)
    }

    #[inline(always)]
    fn decode_plan<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<DecodePlan, Error> {
        let wire_len = F::decode_usize(input, scratch)?;
        Ok(DecodePlan {
            output_cap: S::decoded_max_len(wire_len)?,
            wire_len,
            exact_len: None,
        })
    }
}

pub struct AsciiLength<const N: usize>;

impl<const N: usize, S: Step> LengthSpec<S> for AsciiLength<N> {
    #[inline(always)]
    fn encoded_len(_semantic_len: usize, _wire_len: usize) -> Result<usize, Error> {
        Ok(N)
    }

    #[inline(always)]
    fn encode(output: &mut &mut [u8], _scratch: &mut &mut [u8], semantic_len: usize, _wire_len: usize) -> Result<(), Error> {
        encode_decimal_ascii_fixed(output, semantic_len, N)
    }

    #[inline(always)]
    fn decode_plan<'a>(input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<DecodePlan, Error> {
        let semantic_len = decode_decimal_ascii_fixed(input, N)?;
        let wire_len = S::encoded_len(semantic_len)?;
        Ok(DecodePlan {
            output_cap: S::decoded_max_len(wire_len)?,
            wire_len,
            exact_len: Some(semantic_len),
        })
    }
}

pub struct AsciiWireLength<const N: usize>;

impl<const N: usize, S: Step> LengthSpec<S> for AsciiWireLength<N> {
    #[inline(always)]
    fn encoded_len(_semantic_len: usize, _wire_len: usize) -> Result<usize, Error> {
        Ok(N)
    }

    #[inline(always)]
    fn encode(output: &mut &mut [u8], _scratch: &mut &mut [u8], _semantic_len: usize, wire_len: usize) -> Result<(), Error> {
        encode_decimal_ascii_fixed(output, wire_len, N)
    }

    #[inline(always)]
    fn decode_plan<'a>(input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<DecodePlan, Error> {
        let wire_len = decode_decimal_ascii_fixed(input, N)?;
        Ok(DecodePlan {
            output_cap: S::decoded_max_len(wire_len)?,
            wire_len,
            exact_len: None,
        })
    }
}

pub struct EbcdicLength<const N: usize>;

impl<const N: usize, S: Step> LengthSpec<S> for EbcdicLength<N> {
    #[inline(always)]
    fn encoded_len(_semantic_len: usize, _wire_len: usize) -> Result<usize, Error> {
        Ok(N)
    }

    #[inline(always)]
    fn encode(output: &mut &mut [u8], _scratch: &mut &mut [u8], semantic_len: usize, _wire_len: usize) -> Result<(), Error> {
        encode_decimal_ebcdic_fixed(output, semantic_len, N)
    }

    #[inline(always)]
    fn decode_plan<'a>(input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<DecodePlan, Error> {
        let semantic_len = decode_decimal_ebcdic_fixed(input, N)?;
        let wire_len = S::encoded_len(semantic_len)?;
        Ok(DecodePlan {
            output_cap: S::decoded_max_len(wire_len)?,
            wire_len,
            exact_len: Some(semantic_len),
        })
    }
}

pub struct BlankableEbcdicLength<const N: usize>;

impl<const N: usize, S: Step> LengthSpec<S> for BlankableEbcdicLength<N> {
    #[inline(always)]
    fn encoded_len(_semantic_len: usize, _wire_len: usize) -> Result<usize, Error> {
        Ok(N)
    }

    #[inline(always)]
    fn encode(output: &mut &mut [u8], _scratch: &mut &mut [u8], semantic_len: usize, _wire_len: usize) -> Result<(), Error> {
        encode_decimal_ebcdic_blankable_fixed(output, semantic_len, N)
    }

    #[inline(always)]
    fn decode_plan<'a>(input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<DecodePlan, Error> {
        let semantic_len = decode_decimal_ebcdic_blankable_fixed(input, N)?;
        let wire_len = S::encoded_len(semantic_len)?;
        Ok(DecodePlan {
            output_cap: S::decoded_max_len(wire_len)?,
            wire_len,
            exact_len: Some(semantic_len),
        })
    }
}

pub struct EbcdicWireLength<const N: usize>;

impl<const N: usize, S: Step> LengthSpec<S> for EbcdicWireLength<N> {
    #[inline(always)]
    fn encoded_len(_semantic_len: usize, _wire_len: usize) -> Result<usize, Error> {
        Ok(N)
    }

    #[inline(always)]
    fn encode(output: &mut &mut [u8], _scratch: &mut &mut [u8], _semantic_len: usize, wire_len: usize) -> Result<(), Error> {
        encode_decimal_ebcdic_fixed(output, wire_len, N)
    }

    #[inline(always)]
    fn decode_plan<'a>(input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<DecodePlan, Error> {
        let wire_len = decode_decimal_ebcdic_fixed(input, N)?;
        Ok(DecodePlan {
            output_cap: S::decoded_max_len(wire_len)?,
            wire_len,
            exact_len: None,
        })
    }
}

pub struct Rest;

impl<S: Step> LengthSpec<S> for Rest {
    #[inline(always)]
    fn encoded_len(_semantic_len: usize, _wire_len: usize) -> Result<usize, Error> {
        Ok(0)
    }

    #[inline(always)]
    fn encode(_output: &mut &mut [u8], _scratch: &mut &mut [u8], _semantic_len: usize, _wire_len: usize) -> Result<(), Error> {
        Ok(())
    }

    #[inline(always)]
    fn decode_plan<'a>(input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<DecodePlan, Error> {
        let wire_len = input.len();
        Ok(DecodePlan {
            output_cap: S::decoded_max_len(wire_len)?,
            wire_len,
            exact_len: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{AsciiLength, AsciiWireLength, BlankableEbcdicLength, EbcdicLength, EbcdicWireLength, LengthSpec, WireFixed};
    use crate::field::Identity;

    fn encode_length<L: LengthSpec<Identity>>(semantic_len: usize, wire_len: usize) -> Vec<u8> {
        let mut output = [0u8; 16];
        let mut scratch = [0u8; 16];
        let total = output.len();
        let used = {
            let mut out = output.as_mut_slice();
            let mut scratch = scratch.as_mut_slice();
            L::encode(&mut out, &mut scratch, semantic_len, wire_len).unwrap();
            total - out.len()
        };
        output[..used].to_vec()
    }

    fn decode_semantic<L: LengthSpec<Identity>>(input: &[u8]) -> (usize, usize, Option<usize>) {
        let mut input = input;
        let mut scratch = [];
        let mut scratch_ptr = scratch.as_mut_slice();
        let plan = L::decode_plan(&mut input, &mut scratch_ptr).unwrap();
        (plan.output_cap, plan.wire_len, plan.exact_len)
    }

    #[test]
    fn test_ascii_length_specs() {
        assert_eq!(encode_length::<AsciiLength<2>>(16, 0), b"16");
        assert_eq!(encode_length::<AsciiLength<3>>(255, 0), b"255");
        assert_eq!(encode_length::<AsciiLength<4>>(9999, 0), b"9999");
        assert_eq!(decode_semantic::<AsciiLength<2>>(b"16"), (16, 16, Some(16)));
        assert_eq!(decode_semantic::<AsciiWireLength<2>>(b"19"), (19, 19, None));
    }

    #[test]
    fn test_ebcdic_length_specs() {
        assert_eq!(encode_length::<EbcdicLength<2>>(16, 0), [0xF1, 0xF6]);
        assert_eq!(encode_length::<EbcdicLength<3>>(255, 0), [0xF2, 0xF5, 0xF5]);
        assert_eq!(encode_length::<EbcdicLength<4>>(9999, 0), [0xF9, 0xF9, 0xF9, 0xF9]);
        assert_eq!(decode_semantic::<EbcdicLength<2>>(&[0xF1, 0xF6]), (16, 16, Some(16)));
        assert_eq!(decode_semantic::<EbcdicWireLength<2>>(&[0xF1, 0xF9]), (19, 19, None));
        assert_eq!(encode_length::<BlankableEbcdicLength<2>>(0, 0), [0x40, 0x40]);
        assert_eq!(encode_length::<BlankableEbcdicLength<2>>(16, 0), [0xF1, 0xF6]);
        assert_eq!(decode_semantic::<BlankableEbcdicLength<2>>(&[0x40, 0x40]), (0, 0, Some(0)));
        assert_eq!(decode_semantic::<BlankableEbcdicLength<2>>(&[0xF1, 0xF6]), (16, 16, Some(16)));
    }

    #[test]
    fn test_length_overflow_and_invalid() {
        let mut output = [0u8; 2];
        let mut out = output.as_mut_slice();
        let mut scratch = [0u8; 0];
        assert!(<AsciiLength<2> as LengthSpec<Identity>>::encode(&mut out, &mut scratch.as_mut_slice(), 100, 0).is_err());

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
        assert_eq!(plan.output_cap, 7);
        assert_eq!(plan.wire_len, 7);
        assert_eq!(plan.exact_len, None);
    }
}
