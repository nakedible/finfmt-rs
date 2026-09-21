use core::marker::PhantomData;

use super::{Check, LengthSpec, Step};
use crate::primitive::bytes::{decode_padded_bytes, reserve_filled_area};
use crate::utils::cold_path;
use crate::{Error, ScalarFmt};

/// Compose a semantic check, length framing, and byte transform.
/// `C` and `S` must agree on the logical length unit and input repertoire.
pub struct Field<C, L, S = super::Identity>(PhantomData<(C, L, S)>);
pub struct PaddedField<C, L, S, const PAD_TO: usize, const FILL: u8>(PhantomData<(C, L, S)>);

impl<C: Check, L: LengthSpec<S>, S: Step> ScalarFmt for Field<C, L, S> {
    #[inline(always)]
    fn encoded_len(input: &[u8]) -> Result<usize, Error> {
        let semantic_len = C::validate(input)?;
        let wire_len = S::encoded_len(semantic_len)?;
        L::encoded_len(semantic_len, wire_len)?.checked_add(wire_len).ok_or_else(|| {
            cold_path();
            Error::BufferOverflow
        })
    }

    fn encode(output: &mut &mut [u8], scratch: &mut &mut [u8], input: &[u8]) -> Result<(), Error> {
        let semantic_len = C::validate(input)?;
        let wire_len = S::encoded_len(semantic_len)?;
        L::encode(output, scratch, semantic_len, wire_len)?;

        let encoded = S::encode(output, scratch, input)?;
        debug_assert_eq!(encoded.len(), wire_len, "step encoded a different number of bytes than predicted");
        Ok(())
    }

    fn decode<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<&'a [u8], Error> {
        let plan = L::decode_plan(input, scratch)?;
        let wire = input.split_off(..plan.wire_len).ok_or_else(|| {
            cold_path();
            Error::UnexpectedEof
        })?;
        let semantic = S::decode(wire, scratch, plan.semantic_len)?;
        let semantic_len = C::validate(semantic)?;
        if let Some(expected_len) = plan.semantic_len
            && semantic_len != expected_len
        {
            cold_path();
            return Err(Error::Invalid);
        }
        Ok(semantic)
    }
}

impl<C: Check, L: LengthSpec<S>, S: Step, const PAD_TO: usize, const FILL: u8> ScalarFmt for PaddedField<C, L, S, PAD_TO, FILL> {
    #[inline(always)]
    fn encoded_len(input: &[u8]) -> Result<usize, Error> {
        let semantic_len = C::validate(input)?;
        let wire_len = S::encoded_len(semantic_len)?;
        if wire_len > PAD_TO {
            cold_path();
            return Err(Error::Invalid);
        }
        L::encoded_len(semantic_len, wire_len)?.checked_add(PAD_TO).ok_or_else(|| {
            cold_path();
            Error::BufferOverflow
        })
    }

    fn encode(output: &mut &mut [u8], scratch: &mut &mut [u8], input: &[u8]) -> Result<(), Error> {
        let semantic_len = C::validate(input)?;
        let wire_len = S::encoded_len(semantic_len)?;
        if wire_len > PAD_TO {
            cold_path();
            return Err(Error::Invalid);
        }
        L::encode(output, scratch, semantic_len, wire_len)?;
        let area = reserve_filled_area(output, PAD_TO, FILL)?;
        let (field, _tail) = area.split_at_mut(wire_len);
        let mut field_out = field;
        S::encode(&mut field_out, scratch, input)?;
        if !field_out.is_empty() {
            cold_path();
            return Err(Error::Internal);
        }
        Ok(())
    }

    fn decode<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<&'a [u8], Error> {
        let plan = L::decode_plan(input, scratch)?;
        if plan.wire_len > PAD_TO {
            cold_path();
            return Err(Error::Invalid);
        }
        let wire = decode_padded_bytes(input, PAD_TO, plan.wire_len, FILL)?;
        let semantic = S::decode(wire, scratch, plan.semantic_len)?;
        let semantic_len = C::validate(semantic)?;
        if let Some(expected_len) = plan.semantic_len
            && semantic_len != expected_len
        {
            cold_path();
            return Err(Error::Invalid);
        }
        Ok(semantic)
    }
}

#[cfg(test)]
mod tests {
    use super::{Field, PaddedField};
    use crate::field::{
        Ascii, Ebcdic1142, Ebcdic1142Text, EbcdicLength, EbcdicWireLength, Fixed, FixedBinaryBe, FixedNibbleInt, Length, MinusPrefix,
        Numeric, PackNibblesLeft, PackNibblesRight, PadLeft, PadLeftEven, PadRight, PadRightEven, SignPrefix, Track2,
    };
    use crate::primitive::nibble::{Bcdz, HexEbcdic, HexUpper};
    use crate::{AsciiLength, Ebcdic037, Error, Identity, ScalarFmt, WireFixed, WireLength};

    type LlvarPan = Field<Numeric<0, 19>, EbcdicLength<2>, crate::chain!(PadRight<19, b'?'>, PadLeftEven<b'0'>, PackNibblesRight<Bcdz, 0>)>;
    type LlvarTrack2 = Field<Track2<0, 37>, EbcdicWireLength<2>, crate::chain!(PadRightEven<b'?'>, PackNibblesLeft<Bcdz, 0x0F>)>;
    type LlvarHexAscii = Field<Ascii<0, 2>, EbcdicLength<2>, crate::chain!(crate::Ebcdic037, PackNibblesRight<HexEbcdic, 0>)>;
    type N16 = Field<Numeric<1, 16>, EbcdicLength<2>, crate::chain!(PadLeft<16, b'0'>, PackNibblesRight<Bcdz, 0>)>;
    type CdAmount = SignPrefix<N16>;
    type PlusMinusAmount = SignPrefix<N16, b'+', b'-'>;
    type MinusAmount = MinusPrefix<N16>;
    type HexLenAscii = Field<Ascii<0, 255>, Length<FixedNibbleInt<HexUpper, 2>>>;
    type BinaryLenAscii = Field<Ascii<0, 255>, Length<FixedBinaryBe<1>>>;
    type FixedEbcdicNumeric2 = Field<
        Numeric<1, 2>,
        crate::WireFixed<2>,
        crate::chain!(PadLeft<2, b'0', 1>, crate::DecodeCheck<crate::Ebcdic037, crate::EbcdicPrintable<2, 2>>),
    >;
    type FixedIbm1142<const N: usize> = Field<Ebcdic1142Text<0, N>, Fixed<N>, crate::chain!(Ebcdic1142, PadRight<N, 0x40>)>;
    type PaddedHex = PaddedField<crate::HexUpperEven<0, 8>, EbcdicWireLength<2>, PackNibblesRight<HexUpper, 0>, 4, 0x40>;
    type FixedAsciiViaEbcdic = Field<Ascii<1, 1>, Fixed<1>, crate::Ebcdic037>;
    type StrictFixedAsciiViaEbcdic = Field<Ascii<1, 1>, Fixed<1>, crate::DecodeCheck<crate::Ebcdic037, crate::EbcdicPrintable<1, 1>>>;

    fn encode_field<F: ScalarFmt>(input: &[u8], out_len: usize, scratch_len: usize) -> Result<Vec<u8>, Error> {
        let mut out = vec![0u8; out_len];
        let mut scratch = vec![0u8; scratch_len];
        let total = out.len();
        let used = {
            let mut out_ptr = out.as_mut_slice();
            let mut scratch_ptr = scratch.as_mut_slice();
            F::encode(&mut out_ptr, &mut scratch_ptr, input)?;
            total - out_ptr.len()
        };
        Ok(out[..used].to_vec())
    }

    fn decode_field<F: ScalarFmt>(input: &[u8], scratch_len: usize) -> Result<Vec<u8>, Error> {
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
            let mut scratch_ptr = scratch.as_mut_slice();
            F::encode_str(&mut out_ptr, &mut scratch_ptr, input)?;
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
    fn test_llvar_pan_encode_decode() {
        let encoded = encode_field::<LlvarPan>(b"1234567890123456", LlvarPan::encoded_len(b"1234567890123456").unwrap(), 64).unwrap();
        assert_eq!(encoded, b"\xF1\xF6\x01\x23\x45\x67\x89\x01\x23\x45\x6F\xFF");
        assert_eq!(decode_field::<LlvarPan>(&encoded, 64).unwrap(), b"1234567890123456");
    }

    #[test]
    fn test_llvar_pan_buffer_overflow() {
        assert_eq!(encode_field::<LlvarPan>(b"1234567890123456", 11, 19), Err(Error::BufferOverflow));
        assert_eq!(
            decode_field::<LlvarPan>(b"\xF1\xF6\x01\x23\x45\x67\x89\x01\x23\x45\x6F\xFF", 18),
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
        assert_eq!(encoded, b"D\xF0\xF5\x00\x00\x00\x00\x00\x01\x23\x45");
        assert_eq!(decode_field::<CdAmount>(&encoded, 64).unwrap(), b"-12345");
        let encoded = encode_field::<CdAmount>(b"12345", CdAmount::encoded_len(b"12345").unwrap(), 32).unwrap();
        assert_eq!(encoded, b"C\xF0\xF5\x00\x00\x00\x00\x00\x01\x23\x45");
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
        let mut scratch_ptr = &mut scratch[..];
        CdAmount::encode_i64(&mut out_ptr, &mut scratch_ptr, -42).unwrap();
        let used = total - out_ptr.len();
        let mut input = &out[..used];
        let mut scratch_ptr = &mut scratch[..];
        assert_eq!(CdAmount::decode_i64(&mut input, &mut scratch_ptr).unwrap(), -42);
    }

    #[test]
    fn test_sign_prefix_plus_minus() {
        let encoded = encode_field::<PlusMinusAmount>(b"-42", PlusMinusAmount::encoded_len(b"-42").unwrap(), 32).unwrap();
        assert_eq!(encoded, b"-\xF0\xF2\x00\x00\x00\x00\x00\x00\x00\x42");
        assert_eq!(decode_field::<PlusMinusAmount>(&encoded, 64).unwrap(), b"-42");
    }

    #[test]
    fn test_minus_prefix_bytes_and_i64() {
        let encoded = encode_field::<MinusAmount>(b"-42", MinusAmount::encoded_len(b"-42").unwrap(), 32).unwrap();
        assert_eq!(encoded, b"-\xF0\xF2\x00\x00\x00\x00\x00\x00\x00\x42");
        assert_eq!(decode_field::<MinusAmount>(&encoded, 64).unwrap(), b"-42");

        let encoded = encode_field::<MinusAmount>(b"42", MinusAmount::encoded_len(b"42").unwrap(), 32).unwrap();
        assert_eq!(encoded, b"\xF0\xF2\x00\x00\x00\x00\x00\x00\x00\x42");
        assert_eq!(decode_field::<MinusAmount>(&encoded, 64).unwrap(), b"42");
        assert_eq!(MinusAmount::encoded_len(b"+42"), Err(Error::Invalid));

        let mut out = [0u8; 32];
        let mut scratch = [0u8; 32];
        let total = out.len();
        let mut out_ptr = &mut out[..];
        let mut scratch_ptr = &mut scratch[..];
        MinusAmount::encode_i64(&mut out_ptr, &mut scratch_ptr, -42).unwrap();
        let used = total - out_ptr.len();
        let mut input = &out[..used];
        let mut scratch_ptr = &mut scratch[..];
        assert_eq!(MinusAmount::decode_i64(&mut input, &mut scratch_ptr).unwrap(), -42);

        let mut out_ptr = &mut out[..];
        let mut scratch_ptr = &mut scratch[..];
        MinusAmount::encode_i64(&mut out_ptr, &mut scratch_ptr, 42).unwrap();
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
        type Permissive = Field<Ascii<0, 99>, crate::Rest, crate::Ebcdic037>;
        type Strict = Field<Ascii<0, 99>, crate::Rest, crate::DecodeCheck<crate::Ebcdic037, crate::Ebcdic037Ascii<0, 99>>>;
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
    fn test_padded_field_roundtrip_and_edges() {
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
        type BorrowedChain = Field<Ascii<0, 8>, WireFixed<8>, crate::chain!(PadLeft<8>, PadRight<8>)>;
        type Padded = PaddedField<Ascii<0, 8>, AsciiLength<1>, Identity, 8, b' '>;
        type Translated = Field<Ascii<3, 3>, Fixed<3>, crate::chain!(Ebcdic037, PadRight<5, 0x40>)>;
        type Utf8 = Field<Ebcdic1142Text<1, 1>, Fixed<1>, Ebcdic1142>;
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
    fn semantic_length_hints_are_enforced_after_decoding() {
        type Mismatched = Field<Ebcdic1142Text<0, 8>, AsciiLength<1>>;
        type Padded = PaddedField<Ebcdic1142Text<0, 8>, AsciiLength<1>, Identity, 8, b' '>;
        assert_eq!(decode::<Mismatched>("3€tail".as_bytes(), 64).0, Err(Error::Invalid));
        assert_eq!(decode::<Padded>("3€     tail".as_bytes(), 64).0, Err(Error::Invalid));
    }

    #[test]
    fn semantic_hint_rejects_nonpadding_excess_for_both_fields() {
        type F = Field<Ascii<0, 4>, AsciiLength<1>, PadRight<4>>;
        type P = PaddedField<Ascii<0, 4>, AsciiLength<1>, PadRight<4>, 5, b' '>;
        assert_eq!(F::decode(&mut &b"1AB  "[..], &mut &mut [][..]), Err(Error::Invalid));
        assert_eq!(P::decode(&mut &b"1AB   "[..], &mut &mut [][..]), Err(Error::Invalid));
    }

    #[test]
    fn chained_borrows_survive_later_fields() {
        type F = Field<Ascii<0, 8>, AsciiLength<1>, crate::chain!(PadRight<4>, Ebcdic037)>;
        let mut wire = &b"1\xC1\x40\x40\x402\xC2\xC3\x40\x40"[..];
        let mut scratch = [0; 64];
        let mut arena = scratch.as_mut_slice();
        let a = F::decode(&mut wire, &mut arena).unwrap();
        let b = F::decode(&mut wire, &mut arena).unwrap();
        assert_eq!(a, b"A");
        assert_eq!(b, b"BC");
        assert!(wire.is_empty());
        type U = Field<Ebcdic1142Text<0, 8>, AsciiLength<1>, Ebcdic1142>;
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
        type F = Field<Ebcdic1142Text<0, { usize::MAX }>, WireLength<FixedBinaryBe<8>>, Ebcdic1142>;
        let max = (usize::MAX as u64).to_be_bytes();
        assert_eq!(decode_field::<F>(&max, 0), Err(Error::UnexpectedEof));
    }
    #[test]
    #[cfg(debug_assertions)]
    fn incompatible_length_units_are_diagnosed_in_debug() {
        type WrongBytes = Field<Ebcdic1142Text<1, 1>, AsciiLength<2>>;
        type WrongScalars = Field<crate::Binary<1, 3>, AsciiLength<2>, Ebcdic1142>;
        fn fails<F: ScalarFmt>() {
            assert!(
                std::panic::catch_unwind(|| {
                    let mut output = [0; 16];
                    F::encode(&mut &mut output[..], &mut &mut [][..], "€".as_bytes()).unwrap();
                })
                .is_err()
            );
        }
        fails::<WrongBytes>();
        fails::<WrongScalars>();
    }
}
