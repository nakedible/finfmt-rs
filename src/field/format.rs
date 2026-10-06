use core::marker::PhantomData;

use super::{Check, LengthSpec, Step};
use crate::primitive::bytes::take_bytes;
use crate::utils::{cold_path, length_as_invalid};
use crate::{Error, ScalarFmt};

/// Compose a semantic check, length framing, and byte transform.
/// `C` and `S` must agree on the value unit and input repertoire. `L` states
/// the length at the chain's [`crate::Count`] marker, or the wire length
/// without one. A chain with two markers fails to build:
///
/// ```compile_fail
/// use finfmt::{Ascii, AsciiLength, Count, Field, ScalarFmt, chain};
/// type Twice = Field<Ascii<0, 9>, AsciiLength<1>, chain!(Count, Count)>;
/// let _ = Twice::encoded_len(b"A");
/// ```
pub struct Field<C, L, S = super::Identity>(PhantomData<(C, L, S)>);

impl<C: Check, L: LengthSpec, S: Step> Field<C, L, S> {
    #[inline(always)]
    fn total_len(semantic_len: usize) -> Result<usize, Error> {
        const { S::HAS_COUNT };
        let wire_len = S::encoded_len(semantic_len)?;
        framed(L::encoded_len(S::counted_len(semantic_len)?))?
            .checked_add(wire_len)
            .ok_or_else(|| {
                cold_path();
                Error::BufferOverflow
            })
    }

    #[inline(always)]
    fn encode_checked(output: &mut &mut [u8], scratch: &mut [u8], input: &[u8], semantic_len: usize) -> Result<(), Error> {
        framed(encode_length::<L, S>(output, scratch, semantic_len))?;
        encode_steps::<S>(output, scratch, input, semantic_len)
    }

    /// Frame and step-decode a value, without checking it.
    #[inline(always)]
    fn decode_unchecked<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<&'a [u8], Error> {
        decode_framed::<L, S>(input, scratch)
    }
}

/// Write the length `L` states for `semantic_len` bytes encoded through `S`:
/// the count at the chain's marker, or the wire length without one.
#[inline(always)]
pub(crate) fn encode_length<L: LengthSpec, S: Step>(output: &mut &mut [u8], scratch: &mut [u8], semantic_len: usize) -> Result<(), Error> {
    const { S::HAS_COUNT };
    L::encode(output, scratch, S::counted_len(semantic_len)?)
}

/// Encode `input`, whose length in the chain's units is `semantic_len`,
/// through `S`.
#[inline(always)]
pub(crate) fn encode_steps<S: Step>(output: &mut &mut [u8], scratch: &mut [u8], input: &[u8], semantic_len: usize) -> Result<(), Error> {
    let encoded = S::encode(output, scratch, input)?;
    debug_assert_eq!(
        Ok(encoded.len()),
        S::encoded_len(semantic_len),
        "step encoded a different number of bytes than predicted"
    );
    Ok(())
}

/// Read the length `L` states, take the extent it gives, and decode it
/// through `S`.
#[inline(always)]
pub(crate) fn decode_framed<'a, L: LengthSpec, S: Step>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<&'a [u8], Error> {
    const { S::HAS_COUNT };
    let count = L::decode(input, scratch)?;
    let wire = match count {
        // The count is untrusted: a wire length it overflows is invalid data.
        Some(count) => take_bytes(input, S::counted_wire_len(count).map_err(overflow_as_invalid)?)?,
        None => core::mem::take(input),
    };
    S::decode_counted(wire, scratch, count)
}

#[inline(always)]
fn overflow_as_invalid(error: Error) -> Error {
    cold_path();
    if error == Error::BufferOverflow { Error::Invalid } else { error }
}

/// A field whose check accepts a length its framing cannot hold is written
/// wrong. Debug builds catch it; release passes the framing's error through.
#[inline(always)]
fn framed<T>(result: Result<T, Error>) -> Result<T, Error> {
    debug_assert!(
        !matches!(result, Err(Error::Invalid | Error::InvalidValueLength)),
        "the field's check accepts a length its framing cannot hold"
    );
    result
}

/// Decode text once: the UTF-8 check here is the only one.
#[inline(always)]
fn decoded_text(semantic: &[u8]) -> Result<&str, Error> {
    core::str::from_utf8(semantic).map_err(|_| {
        cold_path();
        Error::Invalid
    })
}

impl<C: Check, L: LengthSpec, S: Step> ScalarFmt for Field<C, L, S> {
    const TAKES_REST: bool = !L::STATES_LEN;

    #[inline(always)]
    fn encoded_len(input: &[u8]) -> Result<usize, Error> {
        Self::total_len(C::validate(input)?)
    }

    #[inline(always)]
    fn encoded_len_str(input: &str) -> Result<usize, Error> {
        Self::total_len(C::validate_str(input)?)
    }

    fn encode(output: &mut &mut [u8], scratch: &mut [u8], input: &[u8]) -> Result<(), Error> {
        Self::encode_checked(output, scratch, input, C::validate(input)?)
    }

    fn encode_str(output: &mut &mut [u8], scratch: &mut [u8], input: &str) -> Result<(), Error> {
        Self::encode_checked(output, scratch, input.as_bytes(), C::validate_str(input)?)
    }

    fn decode<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<&'a [u8], Error> {
        let semantic = Self::decode_unchecked(input, scratch)?;
        C::validate(semantic).map_err(length_as_invalid)?;
        Ok(semantic)
    }

    fn decode_str<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<&'a str, Error> {
        let text = decoded_text(Self::decode_unchecked(input, scratch)?)?;
        C::validate_str(text).map_err(length_as_invalid)?;
        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::Field;
    use crate::field::{
        Ascii, Charset, CharsetText, Count, EbcdicLength, Fixed, FixedBinaryBe, FixedNibbleInt, Length, MinusPrefix, Numeric,
        PackNibblesLeft, PackNibblesRight, PadLeft, PadLeftEven, PadRight, PadRightEven, SignPrefix, Track2,
    };
    use crate::primitive::nibble::{BcdzDigits, EbcdicHexDigits, UpperHexDigits};
    use crate::{AsciiLength, AsciiSubset, Cp037, Cp1142, Error, ScalarFmt};

    // The prefix counts the digits padded to 19, not the packing nibble.
    type LlvarPan = Field<
        Numeric<0, 19>,
        EbcdicLength<2>,
        crate::chain!(PadRight<19, b'?'>, Count, PadLeftEven<b'0'>, PackNibblesRight<BcdzDigits, 0>),
    >;
    type LlvarTrack2 = Field<Track2<0, 37>, EbcdicLength<2>, crate::chain!(PadRightEven<b'?'>, PackNibblesLeft<BcdzDigits, 0x0F>)>;
    type LlvarHexAscii =
        Field<Ascii<0, 2>, EbcdicLength<2>, crate::chain!(Count, crate::AsciiSubset<crate::Cp037>, PackNibblesRight<EbcdicHexDigits, 0>)>;
    type N16 = Field<Numeric<1, 16>, EbcdicLength<2>, crate::chain!(PadLeft<16, b'0', 1>, Count, PackNibblesRight<BcdzDigits, 0>)>;
    type CdAmount = SignPrefix<N16>;
    type PlusMinusAmount = SignPrefix<N16, b'+', b'-'>;
    type MinusAmount = MinusPrefix<N16>;
    type HexLenAscii = Field<Ascii<0, 255>, Length<FixedNibbleInt<UpperHexDigits, 2>>>;
    type BinaryLenAscii = Field<Ascii<0, 255>, Length<FixedBinaryBe<1>>>;
    type FixedEbcdicNumeric2 = Field<
        Numeric<1, 2>,
        Fixed<2>,
        crate::chain!(PadLeft<2, b'0', 1>, crate::DecodeCheck<crate::AsciiSubset<crate::Cp037>, crate::EbcdicPrintable<2, 2>>),
    >;
    type FixedIbm1142<const N: usize> = Field<CharsetText<Cp1142, 0, N>, Fixed<N>, crate::chain!(Charset<Cp1142>, PadRight<N, 0x40>)>;
    // The prefix counts the packed bytes, not the area's fill.
    type PaddedHex =
        Field<crate::UpperHexEven<0, 8>, EbcdicLength<2>, crate::chain!(PackNibblesRight<UpperHexDigits, 0>, Count, PadRight<4, 0x40>)>;
    type FixedAsciiViaEbcdic = Field<Ascii<1, 1>, Fixed<1>, crate::AsciiSubset<crate::Cp037>>;
    type StrictFixedAsciiViaEbcdic =
        Field<Ascii<1, 1>, Fixed<1>, crate::DecodeCheck<crate::AsciiSubset<crate::Cp037>, crate::EbcdicPrintable<1, 1>>>;

    fn encode_field<F: ScalarFmt>(input: &[u8], out_len: usize, scratch_len: usize) -> Result<Vec<u8>, Error> {
        let mut out = vec![0u8; out_len];
        let mut scratch = vec![0u8; scratch_len];
        let total = out.len();
        let used = {
            let mut out_ptr = out.as_mut_slice();
            let scratch_ptr = scratch.as_mut_slice();
            F::encode(&mut out_ptr, scratch_ptr, input)?;
            total - out_ptr.len()
        };
        Ok(out[..used].to_vec())
    }

    pub(super) fn decode_field<F: ScalarFmt>(input: &[u8], scratch_len: usize) -> Result<Vec<u8>, Error> {
        let mut scratch = vec![0u8; scratch_len];
        let mut input_ptr = input;
        let mut scratch_ptr = scratch.as_mut_slice();
        let decoded = F::decode(&mut input_ptr, &mut scratch_ptr)?;
        Ok(decoded.to_vec())
    }

    fn encode_field_str<F: ScalarFmt>(input: &str, out_len: usize, scratch_len: usize) -> Result<Vec<u8>, Error> {
        let mut out = vec![0u8; out_len];
        let mut scratch = vec![0u8; scratch_len];
        let total = out.len();
        let used = {
            let mut out_ptr = out.as_mut_slice();
            let scratch_ptr = scratch.as_mut_slice();
            F::encode_str(&mut out_ptr, scratch_ptr, input)?;
            total - out_ptr.len()
        };
        Ok(out[..used].to_vec())
    }

    fn decode_field_str<F: ScalarFmt>(input: &[u8], scratch_len: usize) -> Result<String, Error> {
        let mut scratch = vec![0u8; scratch_len];
        let mut input_ptr = input;
        let mut scratch_ptr = scratch.as_mut_slice();
        Ok(F::decode_str(&mut input_ptr, &mut scratch_ptr)?.to_owned())
    }

    #[test]
    fn text_checks_see_strings_once() {
        use crate::Check;
        type Text = Field<CharsetText<Cp1142, 1, 3>, AsciiLength<1>, Charset<Cp1142>>;
        let mut wire = [0; 8];
        let mut out = wire.as_mut_slice();
        Text::encode_str(&mut out, &mut [][..], "ÆØÅ").unwrap();
        assert_eq!(Text::encoded_len_str("ÆØÅ"), Ok(4));
        let mut scratch = [0; 16];
        assert_eq!(Text::decode_str(&mut &wire[..4], &mut &mut scratch[..]), Ok("ÆØÅ"));
        assert_eq!(Text::encoded_len_str("ÆØÅÆ"), Err(Error::InvalidValueLength));
        assert_eq!(CharsetText::<Cp1142, 0, 3>::validate(b"\x80"), Err(Error::Invalid));
        assert_eq!(CharsetText::<Cp1142, 0, 3>::validate_str("€"), Ok(1));
    }

    #[test]
    fn even_padding_is_exact_with_a_count_and_bounded_without() {
        fn roundtrip<F: ScalarFmt>(value: &[u8]) -> Result<(Vec<u8>, Vec<u8>), Error> {
            let mut wire = [0; 32];
            let mut out = wire.as_mut_slice();
            F::encode(&mut out, &mut [0; 64][..], value)?;
            let written = 32 - out.len();
            let decoded = F::decode(&mut &wire[..written], &mut &mut [0; 64][..])?.to_vec();
            Ok((wire[..written].to_vec(), decoded))
        }
        type ByteCount = Field<Track2<0, 37>, EbcdicLength<2>, crate::chain!(PadRightEven<b'?'>, PackNibblesLeft<BcdzDigits, 0>)>;
        type DigitCount = Field<Numeric<0, 19>, EbcdicLength<2>, crate::chain!(Count, PadRightEven<b'?'>, PackNibblesLeft<BcdzDigits, 0>)>;
        assert_eq!(
            roundtrip::<ByteCount>(b"123=45"),
            Ok((b"\xF0\xF3\x12\x3D\x45".to_vec(), b"123=45".to_vec()))
        );
        assert_eq!(roundtrip::<ByteCount>(b"123"), Ok((b"\xF0\xF2\x12\x3F".to_vec(), b"123".to_vec())));
        assert_eq!(
            roundtrip::<DigitCount>(b"0000"),
            Ok((b"\xF0\xF4\x00\x00".to_vec(), b"0000".to_vec()))
        );
        // An odd count needs the pad: a zero nibble in its place is invalid.
        assert_eq!(
            DigitCount::decode(&mut &b"\xF0\xF3\x12\x30"[..], &mut &mut [0; 64][..]),
            Err(Error::Invalid)
        );
    }

    #[test]
    fn value_length_errors_only_come_from_encoding() {
        fn encode<F: ScalarFmt>(input: &[u8]) -> Result<(), Error> {
            F::encode(&mut &mut [0; 32][..], &mut [0; 32][..], input)
        }
        fn decode<F: ScalarFmt>(wire: &[u8]) -> Result<Vec<u8>, Error> {
            F::decode(&mut &wire[..], &mut &mut [0; 32][..]).map(<[u8]>::to_vec)
        }
        type Short = Field<Ascii<1, 3>, AsciiLength<1>>;
        assert_eq!(encode::<Short>(b"ABCD"), Err(Error::InvalidValueLength));
        assert_eq!(decode::<Short>(b"4ABCD"), Err(Error::Invalid));
        assert_eq!(encode::<crate::FixedSignedComp3<2>>(b""), Err(Error::InvalidValueLength));
        assert_eq!(encode::<crate::FixedSignedComp3<2>>(b"-"), Err(Error::InvalidValueLength));
        assert_eq!(encode::<SignPrefix<N16>>(b""), Err(Error::InvalidValueLength));
    }

    #[test]
    fn test_llvar_pan_encode_decode() {
        let encoded = encode_field::<LlvarPan>(b"1234567890123456", LlvarPan::encoded_len(b"1234567890123456").unwrap(), 64).unwrap();
        assert_eq!(encoded, b"\xF1\xF9\x01\x23\x45\x67\x89\x01\x23\x45\x6F\xFF");
        assert_eq!(decode_field::<LlvarPan>(&encoded, 64).unwrap(), b"1234567890123456");
    }

    #[test]
    fn test_llvar_pan_buffer_overflow() {
        assert_eq!(encode_field::<LlvarPan>(b"1234567890123456", 11, 19), Err(Error::BufferOverflow));
        assert_eq!(
            decode_field::<LlvarPan>(b"\xF1\xF9\x01\x23\x45\x67\x89\x01\x23\x45\x6F\xFF", 18),
            Err(Error::BufferOverflow)
        );
    }

    #[test]
    fn test_llvar_pan_rejects_invalid_numeric() {
        assert_eq!(encode_field::<LlvarPan>(b"1234A", 12, 19), Err(Error::Invalid));
    }

    #[test]
    fn test_wire_chain_encode_decode() {
        let encoded = encode_field::<LlvarHexAscii>(b"AB", LlvarHexAscii::encoded_len(b"AB").unwrap(), 8).unwrap();
        assert_eq!(encoded, b"\xF0\xF2\xAB");
        assert_eq!(decode_field::<LlvarHexAscii>(&encoded, 8).unwrap(), b"AB");
    }

    #[test]
    fn test_llvar_track2_encode_decode() {
        let encoded = encode_field::<LlvarTrack2>(
            b"1234567890123456=78",
            LlvarTrack2::encoded_len(b"1234567890123456=78").unwrap(),
            64,
        )
        .unwrap();
        assert_eq!(encoded, b"\xF1\xF0\x12\x34\x56\x78\x90\x12\x34\x56\xD7\x8F");
        assert_eq!(decode_field::<LlvarTrack2>(&encoded, 64).unwrap(), b"1234567890123456=78");
    }

    #[test]
    fn test_sign_prefix_cd_bytes() {
        let encoded = encode_field::<CdAmount>(b"-12345", CdAmount::encoded_len(b"-12345").unwrap(), 32).unwrap();
        assert_eq!(encoded, b"D\xF1\xF6\x00\x00\x00\x00\x00\x01\x23\x45");
        assert_eq!(decode_field::<CdAmount>(&encoded, 64).unwrap(), b"-12345");
        let encoded = encode_field::<CdAmount>(b"12345", CdAmount::encoded_len(b"12345").unwrap(), 32).unwrap();
        assert_eq!(encoded, b"C\xF1\xF6\x00\x00\x00\x00\x00\x01\x23\x45");
        assert_eq!(decode_field::<CdAmount>(&encoded, 64).unwrap(), b"12345");
        assert_eq!(CdAmount::encoded_len(b"+12345"), Err(Error::Invalid));
        assert_eq!(encode_field::<CdAmount>(b"+12345", 11, 32), Err(Error::Invalid));
    }

    #[test]
    fn test_sign_prefix_i64() {
        let mut out = [0u8; 32];
        let mut scratch = [0u8; 32];
        let total = out.len();
        let mut out_ptr = &mut out[..];
        let scratch_ptr = &mut scratch[..];
        CdAmount::encode_i64(&mut out_ptr, scratch_ptr, -42).unwrap();
        let used = total - out_ptr.len();
        let mut input = &out[..used];
        let mut scratch_ptr = &mut scratch[..];
        assert_eq!(CdAmount::decode_i64(&mut input, &mut scratch_ptr).unwrap(), -42);
    }

    #[test]
    fn test_sign_prefix_plus_minus() {
        let encoded = encode_field::<PlusMinusAmount>(b"-42", PlusMinusAmount::encoded_len(b"-42").unwrap(), 32).unwrap();
        assert_eq!(encoded, b"-\xF1\xF6\x00\x00\x00\x00\x00\x00\x00\x42");
        assert_eq!(decode_field::<PlusMinusAmount>(&encoded, 64).unwrap(), b"-42");
    }

    #[test]
    fn test_minus_prefix_bytes_and_i64() {
        let encoded = encode_field::<MinusAmount>(b"-42", MinusAmount::encoded_len(b"-42").unwrap(), 32).unwrap();
        assert_eq!(encoded, b"-\xF1\xF6\x00\x00\x00\x00\x00\x00\x00\x42");
        assert_eq!(decode_field::<MinusAmount>(&encoded, 64).unwrap(), b"-42");

        let encoded = encode_field::<MinusAmount>(b"42", MinusAmount::encoded_len(b"42").unwrap(), 32).unwrap();
        assert_eq!(encoded, b"\xF1\xF6\x00\x00\x00\x00\x00\x00\x00\x42");
        assert_eq!(decode_field::<MinusAmount>(&encoded, 64).unwrap(), b"42");
        assert_eq!(MinusAmount::encoded_len(b"+42"), Err(Error::Invalid));

        let mut out = [0u8; 32];
        let mut scratch = [0u8; 32];
        let total = out.len();
        let mut out_ptr = &mut out[..];
        let scratch_ptr = &mut scratch[..];
        MinusAmount::encode_i64(&mut out_ptr, scratch_ptr, -42).unwrap();
        let used = total - out_ptr.len();
        let mut input = &out[..used];
        let mut scratch_ptr = &mut scratch[..];
        assert_eq!(MinusAmount::decode_i64(&mut input, &mut scratch_ptr).unwrap(), -42);

        let mut out_ptr = &mut out[..];
        let scratch_ptr = &mut scratch[..];
        MinusAmount::encode_i64(&mut out_ptr, scratch_ptr, 42).unwrap();
        let used = total - out_ptr.len();
        let mut input = &out[..used];
        let mut scratch_ptr = &mut scratch[..];
        assert_eq!(MinusAmount::decode_i64(&mut input, &mut scratch_ptr).unwrap(), 42);
    }

    #[test]
    fn test_generic_hex_and_binary_length_fields() {
        let encoded = encode_field::<HexLenAscii>(b"ABC", HexLenAscii::encoded_len(b"ABC").unwrap(), 8).unwrap();
        assert_eq!(encoded, b"03ABC");
        assert_eq!(decode_field::<HexLenAscii>(&encoded, 8).unwrap(), b"ABC");

        let encoded = encode_field::<BinaryLenAscii>(b"ABC", BinaryLenAscii::encoded_len(b"ABC").unwrap(), 8).unwrap();
        assert_eq!(encoded, b"\x03ABC");
        assert_eq!(decode_field::<BinaryLenAscii>(&encoded, 8).unwrap(), b"ABC");
    }

    #[test]
    fn test_fixed_ebcdic_numeric_zero_stays_zero() {
        let encoded = encode_field::<FixedEbcdicNumeric2>(b"0", FixedEbcdicNumeric2::encoded_len(b"0").unwrap(), 8).unwrap();
        assert_eq!(encoded, [0xF0, 0xF0]);
        assert_eq!(decode_field::<FixedEbcdicNumeric2>(&encoded, 8).unwrap(), b"0");
    }

    #[test]
    fn test_decode_check_strict_decode_and_encode_passthrough() {
        assert_eq!(decode_field::<FixedAsciiViaEbcdic>(&[0x00], 4).unwrap(), b"\0");
        assert_eq!(decode_field::<StrictFixedAsciiViaEbcdic>(&[0x00], 4), Err(Error::Invalid));
        assert_eq!(
            encode_field::<StrictFixedAsciiViaEbcdic>(b"A", StrictFixedAsciiViaEbcdic::encoded_len(b"A").unwrap(), 4).unwrap(),
            encode_field::<FixedAsciiViaEbcdic>(b"A", FixedAsciiViaEbcdic::encoded_len(b"A").unwrap(), 4).unwrap()
        );
    }

    #[test]
    fn cp037_strict_decoding_is_opt_in() {
        type Permissive = Field<Ascii<0, 99>, crate::Rest, crate::AsciiSubset<crate::Cp037>>;
        type Strict = Field<
            Ascii<0, 99>,
            crate::Rest,
            crate::DecodeCheck<crate::AsciiSubset<crate::Cp037>, crate::AsciiSubsetBytes<crate::Cp037, 0, 99>>,
        >;
        for (wire, ascii) in [
            (&b""[..], &b""[..]),
            (&b"\xC1\xF1\x40"[..], &b"A1 "[..]),
            (&b"\0\x3F"[..], &b"\0\x1A"[..]),
        ] {
            assert_eq!(decode_field::<Strict>(wire, 128), Ok(ascii.to_vec()));
            assert_eq!(decode_field::<Permissive>(wire, 128), Ok(ascii.to_vec()));
            assert_eq!(encode_field::<Strict>(ascii, 128, 128), Ok(wire.to_vec()));
            assert_eq!(encode_field::<Permissive>(ascii, 128, 128), Ok(wire.to_vec()));
        }
        assert_eq!(decode_field_str::<Permissive>(&[0xC1, 0x4A], 128), Ok("A\x1A".to_owned()));
        assert_eq!(decode_field_str::<Strict>(&[0xC1, 0x4A], 128), Err(Error::Invalid));
        assert_eq!(encode_field_str::<Permissive>("¢", 128, 128), Err(Error::Invalid));
        assert_eq!(encode_field_str::<Strict>("¢", 128, 128), Err(Error::Invalid));
    }

    #[test]
    fn test_ibm1142_string_field_roundtrip() {
        let text = "ABCÆØÅæøå€";
        let encoded = encode_field_str::<FixedIbm1142<10>>(text, FixedIbm1142::<10>::encoded_len_str(text).unwrap(), 64).unwrap();
        assert_eq!(encoded, b"\xC1\xC2\xC3\x7B\x7C\x5B\xC0\x6A\xD0\x5A");
        assert_eq!(decode_field_str::<FixedIbm1142<10>>(&encoded, 64).unwrap(), text);
        assert_eq!(encode_field_str::<FixedIbm1142<10>>("emoji: 😀", 10, 64), Err(Error::Invalid));
    }

    #[test]
    fn uncounted_area_padding_roundtrip_and_edges() {
        let encoded = encode_field::<PaddedHex>(b"ABCD", PaddedHex::encoded_len(b"ABCD").unwrap(), 8).unwrap();
        assert_eq!(encoded, [0xF0, 0xF2, 0xAB, 0xCD, 0x40, 0x40]);
        assert_eq!(decode_field::<PaddedHex>(&encoded, 8).unwrap(), b"ABCD");
        assert_eq!(
            encode_field::<PaddedHex>(b"", PaddedHex::encoded_len(b"").unwrap(), 8).unwrap(),
            [0xF0, 0xF0, 0x40, 0x40, 0x40, 0x40]
        );
        assert_eq!(decode_field::<PaddedHex>(&[0xF0, 0xF0, 0x40, 0x40, 0x40, 0x40], 8).unwrap(), b"");
        assert_eq!(
            decode_field::<PaddedHex>(&[0xF0, 0xF2, 0xAB, 0xCD, 0x40, 0x41], 8),
            Err(Error::Invalid)
        );
        assert_eq!(encode_field::<PaddedHex>(b"ABCDEF0123", 8, 8), Err(Error::InvalidValueLength));
    }

    fn decode<F: ScalarFmt>(wire: &[u8], capacity: usize) -> (Result<Vec<u8>, Error>, usize, usize) {
        let mut input = wire;
        let mut scratch = vec![0; capacity];
        let mut work = scratch.as_mut_slice();
        let result = F::decode(&mut input, &mut work).map(<[u8]>::to_vec);
        (result, input.len(), capacity - work.len())
    }

    #[test]
    fn borrowing_and_transforming_decoders_own_their_scratch_reservations() {
        type Borrowed = Field<Ascii<3, 3>, Fixed<3>>;
        type BorrowedChain = Field<Ascii<0, 8>, Fixed<8>, crate::chain!(PadLeft<8>, PadRight<8>)>;
        type Padded = Field<Ascii<0, 8>, AsciiLength<1>, crate::chain!(Count, PadRight<8>)>;
        type Translated = Field<Ascii<3, 3>, Fixed<5>, crate::chain!(AsciiSubset<Cp037>, PadRight<5, 0x40>)>;
        type Utf8 = Field<CharsetText<Cp1142, 1, 1>, Fixed<1>, Charset<Cp1142>>;
        assert_eq!(decode::<Borrowed>(b"ABCtail", 0), (Ok(b"ABC".to_vec()), 4, 0));
        assert_eq!(decode::<BorrowedChain>(b"     ABCtail", 0), (Ok(b"ABC".to_vec()), 4, 0));
        assert_eq!(decode::<Padded>(b"3ABC     tail", 0), (Ok(b"ABC".to_vec()), 4, 0));
        assert_eq!(
            decode::<Translated>(&[0xC1, 0xC2, 0xC3, 0x40, 0x40], 3),
            (Ok(b"ABC".to_vec()), 0, 3)
        );
        assert_eq!(decode::<Utf8>(&[0xC1], 1), (Ok(b"A".to_vec()), 0, 1));
        assert_eq!(decode::<Utf8>(&[0x5A], 3), (Ok("€".as_bytes().to_vec()), 0, 3));
        assert_eq!(decode::<Utf8>(&[0x5A], 2).0, Err(Error::BufferOverflow));
        let wire = b"ABCtail";
        let mut input = &wire[..];
        let decoded = Borrowed::decode(&mut input, &mut &mut [][..]).unwrap();
        assert_eq!(decoded.as_ptr(), wire.as_ptr());
    }

    #[test]
    fn the_count_marker_decides_what_the_length_counts() {
        // Without a marker the prefix counts wire bytes, padding included.
        type Wire = Field<Ascii<0, 4>, AsciiLength<1>, PadRight<4>>;
        assert_eq!(encode_field::<Wire>(b"AB", 5, 8), Ok(b"4AB  ".to_vec()));
        let mut input = &b"1AB  "[..];
        assert_eq!(Wire::decode(&mut input, &mut &mut [][..]), Ok(&b"A"[..]));
        assert_eq!(input, b"B  ");
        // Before the padding it counts the value: the padding is split off
        // exactly, so a value ending in the fill survives, and a byte other
        // than the fill is invalid.
        type Value = Field<Ascii<0, 4>, AsciiLength<1>, crate::chain!(Count, PadRight<4>)>;
        assert_eq!(encode_field::<Value>(b"AB", 5, 8), Ok(b"2AB  ".to_vec()));
        assert_eq!(encode_field::<Value>(b"A ", 5, 8), Ok(b"2A   ".to_vec()));
        assert_eq!(decode_field::<Value>(b"2A   ", 0), Ok(b"A ".to_vec()));
        assert_eq!(decode_field::<Value>(b"1AB  ", 0), Err(Error::Invalid));
        assert_eq!(decode_field::<Value>(b"6AB    ", 0), Err(Error::Invalid));
        // A fixed length is taken at the marker too: 3 digits in 2 bytes.
        type Packed = Field<Numeric<3, 3>, Fixed<3>, crate::chain!(Count, PackNibblesRight<BcdzDigits, 0>)>;
        assert_eq!(encode_field::<Packed>(b"123", 2, 8), Ok(b"\x01\x23".to_vec()));
        assert_eq!(decode_field::<Packed>(b"\x01\x23", 8), Ok(b"123".to_vec()));
        assert_eq!(decode_field::<Packed>(b"\x11\x23", 8), Err(Error::Invalid));
    }

    #[test]
    fn chained_borrows_survive_later_fields() {
        type F = Field<Ascii<0, 8>, AsciiLength<1>, crate::chain!(PadRight<4>, AsciiSubset<Cp037>)>;
        let mut wire = &b"4\xC1\x40\x40\x404\xC2\xC3\x40\x40"[..];
        let mut scratch = [0; 64];
        let mut arena = scratch.as_mut_slice();
        let a = F::decode(&mut wire, &mut arena).unwrap();
        let b = F::decode(&mut wire, &mut arena).unwrap();
        assert_eq!(a, b"A");
        assert_eq!(b, b"BC");
        assert!(wire.is_empty());
        type U = Field<CharsetText<Cp1142, 0, 8>, AsciiLength<1>, Charset<Cp1142>>;
        let mut wire = &b"1\x5A1\xC1"[..];
        let euro = U::decode(&mut wire, &mut arena).unwrap();
        let letter = U::decode(&mut wire, &mut arena).unwrap();
        assert_eq!(euro, "€".as_bytes());
        assert_eq!(letter, b"A");
        assert!(wire.is_empty());
        assert_eq!(a, b"A");
        assert_eq!(b, b"BC");
    }

    #[test]
    fn wire_extent_is_framed_before_decoding_capacity() {
        type F = Field<CharsetText<Cp1142, 0, { usize::MAX }>, Length<FixedBinaryBe<8>>, Charset<Cp1142>>;
        type Counted =
            Field<crate::Binary<0, { usize::MAX }>, Length<FixedBinaryBe<8>>, crate::chain!(Count, crate::UnpackNibbles<UpperHexDigits>)>;
        assert_eq!(decode_field::<Counted>(&(usize::MAX as u64).to_be_bytes(), 0), Err(Error::Invalid));
        let max = (usize::MAX as u64).to_be_bytes();
        assert_eq!(decode_field::<F>(&max, 0), Err(Error::UnexpectedEof));
    }
    #[test]
    fn unchecked_ebcdic_1142_rejects_unrepresentable_text() {
        for input in ["😀".as_bytes(), "\u{A4}".as_bytes(), b"\xFF"] {
            let mut output = [0; 8];
            assert_eq!(
                <Charset<Cp1142> as crate::Step>::encode(&mut &mut output[..], &mut [][..], input).map(|_| ()),
                Err(Error::Invalid)
            );
        }
    }

    #[test]
    #[cfg(debug_assertions)]
    fn framing_too_small_for_the_check_is_diagnosed_in_debug() {
        fn fails<F: ScalarFmt>(input: &[u8]) {
            assert!(std::panic::catch_unwind(|| F::encoded_len(input)).is_err());
            assert!(std::panic::catch_unwind(|| F::encode(&mut &mut [0; 512][..], &mut [0; 512][..], input)).is_err());
        }
        fails::<Field<crate::Binary<0, 100>, AsciiLength<1>>>(b"0123456789");
        fails::<Field<Ascii<0, 300>, Length<FixedBinaryBe<1>>>>(&[b'A'; 256]);
        fails::<Field<Ascii<0, 300>, Length<crate::FixedComp3<1>>>>(&[b'A'; 10]);
        // Sizing and encoding both check the padded width against the prefix.
        fails::<Field<Ascii<0, 12>, AsciiLength<1>, PadRight<12>>>(b"AB");
    }

    #[test]
    #[cfg(debug_assertions)]
    fn incompatible_length_units_are_diagnosed_in_debug() {
        type WrongBytes = Field<CharsetText<Cp1142, 1, 1>, AsciiLength<2>>;
        type WrongScalars = Field<crate::Binary<1, 3>, AsciiLength<2>, Charset<Cp1142>>;
        fn fails<F: ScalarFmt>() {
            assert!(
                std::panic::catch_unwind(|| {
                    let mut output = [0; 16];
                    F::encode(&mut &mut output[..], &mut [][..], "€".as_bytes()).unwrap();
                })
                .is_err()
            );
        }
        fails::<WrongBytes>();
        fails::<WrongScalars>();
    }
}

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    use crate::*;

    proptest! {
        #[test]
        fn the_count_marker_can_sit_anywhere_in_the_chain(digits in "[0-9]{0,9}") {
            use crate::primitive::nibble::BcdzDigits;
            type Digits = Field<Numeric<0, 9>, AsciiLength<2>, crate::chain!(Count, PadRightEven<b'?'>, PackNibblesLeft<BcdzDigits>, PadRight<8, 0x40>)>;
            type Even = Field<Numeric<0, 9>, AsciiLength<2>, crate::chain!(PadRightEven<b'?'>, Count, PackNibblesLeft<BcdzDigits>, PadRight<8, 0x40>)>;
            type Bytes = Field<Numeric<0, 9>, AsciiLength<2>, crate::chain!(PadRightEven<b'?'>, PackNibblesLeft<BcdzDigits>, Count, PadRight<8, 0x40>)>;
            fn roundtrip<F: ScalarFmt>(digits: &str) -> Result<u8, TestCaseError> {
                let mut wire = [0; 16];
                let mut out = wire.as_mut_slice();
                F::encode_str(&mut out, &mut [0; 32][..], digits).unwrap();
                let used = 16 - out.len();
                prop_assert_eq!(used, 10);
                let mut scratch = [0; 32];
                prop_assert_eq!(F::decode_str(&mut &wire[..used], &mut &mut scratch[..]), Ok(digits));
                Ok((wire[0] - b'0') * 10 + wire[1] - b'0')
            }
            let len = digits.len() as u8;
            prop_assert_eq!(roundtrip::<Digits>(&digits)?, len);
            prop_assert_eq!(roundtrip::<Even>(&digits)?, len + len % 2);
            prop_assert_eq!(roundtrip::<Bytes>(&digits)?, len.div_ceil(2));
        }

        #[test]
        fn cp1142_field_preserves_values_and_character_prefix(bytes in prop::collection::vec(any::<u8>(), 0..64)) {
            use crate::CodePage;
            type F = Field<CharsetText<Cp1142, 0, 99>, AsciiLength<2>, Charset<Cp1142>>;
            let text: String = bytes.iter().map(|&b| char::from_u32(Cp1142::DECODE[b as usize] as u32).unwrap()).collect();
            let expected = F::encoded_len(text.as_bytes()).unwrap();
            prop_assert_eq!(expected, bytes.len() + 2);
            let mut output = vec![0; expected];
            let mut out = output.as_mut_slice();
            F::encode(&mut out, &mut [][..], text.as_bytes()).unwrap();
            prop_assert!(out.is_empty());
            prop_assert_eq!(&output[2..], &bytes);
            prop_assert_eq!(((output[0] - b'0') * 10 + output[1] - b'0') as usize, bytes.len());
            for size in [text.len(), 1024] {
                prop_assert_eq!(super::tests::decode_field::<F>(&output, size), Ok(text.as_bytes().to_vec()));
            }
        }
    }
}
