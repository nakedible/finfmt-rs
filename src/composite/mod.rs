//! Composite format traits, wrappers, and record macros.
//!
//! Record macros parse field entries syntactically:
//! - `field: Fmt` uses the serde-backed scalar path with `Fmt: ScalarFmt`.
//! - `field: DirectScalar<Fmt>` uses `ScalarValue` directly instead of serde.
//! - `field: Composite<Fmt>` nests another `CompositeFmt`.
//! - `field: Option<...>` means the wire container can omit that field.
//!
//! `Option<Fmt>` is container-level presence, not merely a scalar field whose
//! semantic Rust value happens to be `Option<T>`. In `concat_format!`, optional
//! fields are tail-only and decode to `None` after EOF. In `delimited_format!`,
//! an empty segment decodes to `None`. In bitmap and BER-TLV formats, absence is
//! controlled by the bitmap bit or tag presence. If bytes are always present but
//! a pattern inside those bytes means "no value", express that in the field
//! format, for example with `OptionalAbsent`, not by wrapping the macro field in
//! `Option<...>`.

use core::marker::PhantomData;

use crate::primitive::bytes::{contains_byte, copy_bytes};
use crate::utils::take_scratch;
use crate::{Error, ScalarFmt, StructError};

pub trait CompositeFmt<T> {
    type Decoded<'de>;

    #[inline(always)]
    fn encode(output: &mut &mut [u8], scratch: &mut [u8], value: &T) -> Result<(), StructError> {
        let mut scratch_ptr = scratch;
        Self::encode_cursor(output, &mut scratch_ptr, value)
    }

    #[inline(always)]
    fn decode<'de>(input: &mut &'de [u8], scratch: &'de mut [u8]) -> Result<Self::Decoded<'de>, StructError> {
        let mut scratch_ptr = scratch;
        Self::decode_cursor(input, &mut scratch_ptr)
    }

    fn encode_cursor(output: &mut &mut [u8], scratch: &mut &mut [u8], value: &T) -> Result<(), StructError>;
    fn decode_cursor<'de>(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self::Decoded<'de>, StructError>;
}

/// Encode/decode a composite value using an already available context value.
pub trait ContextFmt<T, C: ?Sized> {
    type Decoded<'de>;

    fn encode_with(output: &mut &mut [u8], scratch: &mut &mut [u8], context: &C, value: &T) -> Result<(), StructError>;
    fn decode_with<'de>(input: &mut &'de [u8], scratch: &mut &'de mut [u8], context: &C) -> Result<Self::Decoded<'de>, StructError>;
}

/// Canonical bytes used by wrapper formats to represent an absent value.
///
/// This is separate from [`CompositeFmt`]: some wrappers only need an explicit
/// absent byte encoding, not a meaningful semantic value.
///
/// Formats using this for `Option<T>` should choose a present-side format that
/// cannot encode the absent bytes unless that lossy mapping is intentional.
pub trait AbsentFmt {
    fn encode_absent(output: &mut &mut [u8], scratch: &mut &mut [u8]) -> Result<(), Error>;

    #[inline(always)]
    fn is_absent(input: &[u8], scratch: &mut &mut [u8]) -> Result<bool, Error> {
        let absent = take_scratch(scratch, input.len())?;
        let mut absent_out = &mut absent[..];
        Self::encode_absent(&mut absent_out, scratch)?;
        if !absent_out.is_empty() {
            crate::utils::cold_path();
            return Err(Error::Internal);
        }
        Ok(input == absent)
    }
}

/// Encode/decode an inner composite through an outer scalar field.
pub struct Frame<F, S>(PhantomData<(F, S)>);
pub struct TrailingLengthFrame<T, Len, Body, Tails, const BASE_LEN: usize>(PhantomData<(T, Len, Body, Tails)>);
pub struct TrailingField<Field, Rest = NoTrailingFields>(PhantomData<(Field, Rest)>);
pub struct NoTrailingFields;
pub struct Empty<T>(PhantomData<T>);
/// Structural BER-TLV representation as a Serde map or sequence of
/// `(tag_hex, value_hex)` pairs.
///
/// Order and duplicates are preserved if the chosen collection type preserves
/// them.
pub struct BerTlvList<T>(PhantomData<T>);
pub struct BoundedList<T, Count, Item, Sep, const MAX: usize>(PhantomData<(T, Count, Item, Sep)>);
pub struct FixedCount<const COUNT: usize>;
/// Encode `None` as an explicit absent byte encoding and decode matching bytes
/// back to `None`.
pub struct OptionalAbsent<T, Inner, Absent, const N: usize>(PhantomData<(T, Inner, Absent)>);
/// Fill the provided absent area with one byte.
pub struct ByteFill<const BYTE: u8 = b' '>;
pub struct FixedAreaList<T, Len, Slot, const MAX: usize>(PhantomData<(T, Len, Slot)>);
pub struct Separator<const BYTE: u8>;

pub type FixedCountList<T, Item, const COUNT: usize> = BoundedList<T, FixedCount<COUNT>, Item, (), COUNT>;

mod bertlv;
mod bertlv_macros;
mod bertlv_serde;
mod bitmap_macros;
mod concat_macros;
mod delimited_macros;
mod repeated;
mod scalar;
mod scalar_serde;
pub use scalar::{Composite, DirectScalar, ScalarValue};
#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde::{Deserialize, Serialize};

    use super::*;
    use crate::field::Length;
    use crate::primitive::nibble::{Bcdz, HexUpper};
    use crate::{
        Ascii, AsciiLength, AsciiWireLength, Binary, Ebcdic037, EbcdicLength, EbcdicWireLength, Error, Field, Fixed, Numeric, PadLeft,
        PadRightEven, SignPrefix, Track2, UnpackNibbles, WireFixed,
    };

    type N6 = Field<Numeric<6, 6>, Fixed<6>>;
    type N2 = Field<Numeric<2, 2>, Fixed<2>>;
    type A2 = Field<Ascii<2, 2>, Fixed<2>>;
    type A3 = Field<Ascii<3, 3>, Fixed<3>>;
    type A4 = Field<crate::Ascii<4, 4>, Fixed<4>>;
    type BitmapBinaryHalfWord = Field<Binary<4, 4>, Fixed<4>>;
    type BitmapBinaryWord = Field<Binary<8, 8>, Fixed<8>>;
    type Track2Fmt = Field<Track2<1, 37>, EbcdicWireLength<2>, crate::chain!(PadRightEven<b'?'>, crate::PackNibblesLeft<Bcdz, 0x0F>)>;
    const PIPE_SEPARATOR: u8 = b'|';

    fn error_kind<T>(result: Result<T, StructError>) -> Result<T, Error> {
        result.map_err(|error| error.kind)
    }
    type AmountFmt = SignPrefix<Field<Numeric<1, 16>, Fixed<16>, crate::chain!(PadLeft<16, b'0'>, crate::PackNibblesRight<Bcdz, 0>)>>;

    #[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    struct FixedTail {
        a: String,
        b: String,
        c: String,
    }

    crate::concat_format! {
        /// Test-only concat format marker with forwarded outer attributes.
        #[allow(missing_docs)]
        struct FixedTailFmt for FixedTail {
            a: N6,
            b: N2,
            c: A4,
        }
    }

    #[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    struct NestedConcat {
        head: String,
        inner: Option<FixedTail>,
        tail: String,
    }

    crate::concat_format! {
        struct NestedConcatFmt for NestedConcat {
            head: N2,
            tail: N2,
            inner: Option<Composite<FixedTailFmt>>,
        }
    }

    type A4Ebcdic = Field<Ascii<4, 4>, Fixed<4>, Ebcdic037>;

    #[derive(Debug, PartialEq, Eq, Serialize)]
    struct BorrowedConcat<'a> {
        ascii: &'a str,
        ebcdic: &'a str,
    }

    crate::concat_format! {
        struct BorrowedConcatFmt for<'a> BorrowedConcat<'a> {
            ascii: A4,
            ebcdic: A4Ebcdic,
        }
    }

    #[derive(Debug, PartialEq, Eq, Serialize)]
    struct BorrowedDelimited<'a> {
        ascii: &'a str,
        ebcdic: &'a str,
    }

    crate::delimited_format! {
        struct BorrowedDelimitedFmt for<'a> BorrowedDelimited<'a>, PIPE_SEPARATOR {
            ascii: A4,
            ebcdic: A4Ebcdic,
        }
    }

    #[derive(Debug, PartialEq, Eq, Serialize)]
    struct BorrowedBitmap<'a> {
        required: &'a str,
        optional: Option<&'a str>,
        ebcdic: Option<&'a str>,
    }

    crate::bitmap_format! {
        struct BorrowedBitmapFmt for<'a> BorrowedBitmap<'a>, crate::bitmap::BitmapLayout::new(1, [None, None, None]), BitmapBinaryHalfWord {
            2 => required: A4,
            3 => optional: Option<A4>,
            4 => ebcdic: Option<A4Ebcdic>,
        }
    }

    #[derive(Debug, PartialEq, Eq, Serialize)]
    struct BorrowedTlv<'a> {
        ascii: &'a str,
        ebcdic: &'a str,
        tail: Option<BorrowedConcat<'a>>,
    }

    crate::ber_tlv_format! {
        struct BorrowedTlvFmt for<'a> BorrowedTlv<'a> {
            "59" => ascii: A4,
            "5A" => ebcdic: A4Ebcdic,
            "DF23" => tail: Option<Composite<BorrowedConcatFmt>>,
        }
    }

    type FramedFixedTailFmt = Frame<Field<Ascii<0, 12>, AsciiLength<2>>, FixedTailFmt>;
    type FramedHexFixedTailFmt = Frame<Field<Binary<0, 12>, AsciiWireLength<2>, UnpackNibbles<HexUpper>>, FixedTailFmt>;
    type OptionalA3SpaceFmt = OptionalAbsent<String, SerdeScalar<A3>, ByteFill<b' '>, 3>;

    #[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    struct FramedConcat {
        head: String,
        inner: FixedTail,
        tail: String,
    }

    crate::concat_format! {
        struct FramedConcatFmt for FramedConcat {
            head: N2,
            inner: Composite<FramedFixedTailFmt>,
            tail: N2,
        }
    }

    #[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    struct FramedHexConcat {
        head: String,
        tail: String,
        inner: FixedTail,
    }

    crate::concat_format! {
        struct FramedHexConcatFmt for FramedHexConcat {
            head: N2,
            tail: N2,
            inner: Composite<FramedHexFixedTailFmt>,
        }
    }

    #[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    struct TrailingLengthData {
        base: String,
        tail1: Option<String>,
        tail2: Option<String>,
    }

    crate::concat_format! {
        struct TrailingLengthBodyFmt for TrailingLengthData {
            base: A2,
            tail1: Composite<OptionalA3SpaceFmt>,
            tail2: Composite<OptionalA3SpaceFmt>,
        }
    }

    type TrailingLengthTails = TrailingField<OptionalA3SpaceFmt, TrailingField<OptionalA3SpaceFmt>>;
    type TrailingLengthDataFmt = TrailingLengthFrame<TrailingLengthData, AsciiLength<2>, TrailingLengthBodyFmt, TrailingLengthTails, 2>;

    #[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    struct TlvData {
        t59_code: String,
        tdf23_tail: Option<FixedTail>,
    }

    crate::ber_tlv_format! {
        #[doc = "Test format for a named BER-TLV struct."]
        struct TlvDataFmt for TlvData {
            "59" => t59_code: A4,
            "DF23" => tdf23_tail: Option<Composite<FixedTailFmt>>,
        }
    }

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct TlvNoDefault {
        t59_code: String,
        tdf23_tail: Option<FixedTail>,
    }

    crate::ber_tlv_format! {
        struct TlvNoDefaultFmt for TlvNoDefault {
            "59" => t59_code: A4,
            "DF23" => tdf23_tail: Option<Composite<FixedTailFmt>>,
        }
    }

    #[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    struct TlvWithExtras {
        t59_code: String,
        #[serde(flatten)]
        extras: BTreeMap<String, String>,
    }

    crate::ber_tlv_format! {
        struct TlvWithExtrasFmt for TlvWithExtras {
            extras: extras,
            "59" => t59_code: A4,
        }
    }

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct TlvWithExtrasNoDefault {
        t59_code: String,
        #[serde(flatten)]
        extras: BTreeMap<String, String>,
    }

    crate::ber_tlv_format! {
        struct TlvWithExtrasNoDefaultFmt for TlvWithExtrasNoDefault {
            extras: extras,
            "59" => t59_code: A4,
        }
    }

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    enum UnionValue {
        Known(String),
        Alpha(String),
        Unknown(String),
    }

    type FixedNumeric4 = Field<Numeric<4, 4>, Fixed<4>>;
    type FixedAlpha4 = Field<crate::Alpha<4, 4>, Fixed<4>>;
    type RestNumeric4 = Field<Numeric<4, 4>, crate::Rest>;
    type RestAscii8 = Field<Ascii<0, 8>, crate::Rest>;

    crate::union_format! {
        #[doc = "Test format for an untagged speculative enum."]
        struct UnionValueFmt for UnionValue {
            Known(DirectScalar<FixedNumeric4>),
            Alpha(DirectScalar<FixedAlpha4>),
            Unknown(DirectScalar<RestAscii8>),
        }
    }

    crate::union_format! {
        struct RestUnionValueFmt for UnionValue {
            Known(DirectScalar<RestNumeric4>),
            Alpha(DirectScalar<FixedAlpha4>),
            Unknown(DirectScalar<RestAscii8>),
        }
    }

    #[derive(Debug, PartialEq, Eq, Serialize)]
    enum BorrowedUnionValue<'a> {
        Known(&'a str),
        Unknown(&'a str),
    }

    crate::union_format! {
        struct BorrowedUnionValueFmt for<'a> BorrowedUnionValue<'a> {
            Known(DirectScalar<FixedNumeric4, &'a str>),
            Unknown(DirectScalar<A4Ebcdic, &'a str>),
        }
    }

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    enum ShortUnionValue {
        Numeric(String),
        Alpha(String),
    }

    crate::union_format! {
        struct ShortUnionValueFmt for ShortUnionValue {
            Numeric(DirectScalar<FixedNumeric4>),
            Alpha(DirectScalar<FixedAlpha4>),
        }
    }

    #[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    struct WithBitmapTlv {
        f003_processing_code: String,
        f048_details: Option<TlvData>,
    }

    crate::bitmap_format! {
        #[doc = "Test format for a nested BER-TLV field inside a bitmap."]
        struct WithBitmapTlvFmt for WithBitmapTlv, crate::bitmap::BitmapLayout::iso(1), BitmapBinaryWord {
            3 => f003_processing_code: N6,
            48 => f048_details: Option<Composite<TlvDataFmt>>,
        }
    }

    #[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    struct Auth1100 {
        f003_processing_code: String,
        f011_stan: String,
        f035_track2_data: Option<String>,
        f048_fixed_tail: Option<FixedTail>,
        f097_amount_net_settlement: Option<i64>,
    }

    crate::bitmap_format! {
        struct Auth1100Fmt for Auth1100, crate::bitmap::BitmapLayout::iso(2), BitmapBinaryWord {
            3 => f003_processing_code: N6,
            11 => f011_stan: N6,
            35 => f035_track2_data: Option<Track2Fmt>,
            48 => f048_fixed_tail: Option<Composite<FixedTailFmt>>,
            97 => f097_amount_net_settlement: Option<AmountFmt>,
        }
    }

    #[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    struct LocalBitmapData {
        a: Option<String>,
        b: Option<String>,
    }

    crate::bitmap_format! {
        struct LocalBitmapDataFmt for LocalBitmapData, crate::bitmap::BitmapLayout::new(1, [None, None, None]), BitmapBinaryHalfWord {
            head: {
                _: A4 = b"HEAD",
            }
            2 => a: Option<N2>,
            3 => b: Option<A4>,
        }
    }

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct BitmapNoDefault {
        head_code: String,
        f002_required: String,
        f003_optional: Option<String>,
    }

    crate::bitmap_format! {
        struct BitmapNoDefaultFmt for BitmapNoDefault, crate::bitmap::BitmapLayout::new(1, [None, None, None]), BitmapBinaryHalfWord {
            head: {
                head_code: N2,
                _: A4 = b"HEAD",
            }
            2 => f002_required: N2,
            3 => f003_optional: Option<A4>,
        }
    }

    #[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(transparent)]
    struct ProcessingCode(String);

    #[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(transparent)]
    struct Stan(u32);

    #[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    struct SerdeScalarBitmapData {
        f003_processing_code: ProcessingCode,
        f011_stan: Option<Stan>,
    }

    crate::bitmap_format! {
        struct SerdeScalarBitmapDataFmt for SerdeScalarBitmapData, crate::bitmap::BitmapLayout::iso(1), BitmapBinaryWord {
            3 => f003_processing_code: N6,
            11 => f011_stan: Option<N6>,
        }
    }

    #[derive(Debug, Default, PartialEq, Eq)]
    struct ManualProcessingCode(String);

    impl ScalarValue for ManualProcessingCode {
        fn encode_with<F: ScalarFmt>(&self, output: &mut &mut [u8], scratch: &mut &mut [u8]) -> Result<(), Error> {
            F::encode_str(output, scratch, &self.0)
        }

        fn decode_with<'de, F: ScalarFmt>(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self, Error> {
            Ok(Self(F::decode_str(input, scratch)?.to_owned()))
        }
    }

    #[derive(Debug, Default, PartialEq, Eq)]
    struct ManualStan(u32);

    impl ScalarValue for ManualStan {
        fn encode_with<F: ScalarFmt>(&self, output: &mut &mut [u8], scratch: &mut &mut [u8]) -> Result<(), Error> {
            F::encode_u64(output, scratch, u64::from(self.0))
        }

        fn decode_with<'de, F: ScalarFmt>(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self, Error> {
            let value = u32::try_from(F::decode_u64(input, scratch)?).map_err(|_| Error::Invalid)?;
            Ok(Self(value))
        }
    }

    #[derive(Debug, Default, PartialEq, Eq)]
    struct ManualScalarBitmapData {
        f003_processing_code: ManualProcessingCode,
        f011_stan: Option<ManualStan>,
    }

    crate::bitmap_format! {
        struct ManualScalarBitmapDataFmt for ManualScalarBitmapData, crate::bitmap::BitmapLayout::iso(1), BitmapBinaryWord {
            3 => f003_processing_code: DirectScalar<N6>,
            11 => f011_stan: Option<DirectScalar<N6>>,
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    struct FieldSyntaxRecord {
        serde_value: Stan,
        direct_value: ManualStan,
        nested_value: FixedTail,
        optional_serde: Option<Stan>,
        optional_direct: Option<ManualStan>,
        optional_nested: Option<FixedTail>,
        optional_direct_inline: Option<ManualStan>,
        optional_nested_inline: Option<FixedTail>,
    }

    crate::concat_format! {
        struct FieldSyntaxConcatFmt for FieldSyntaxRecord {
            serde_value: N2,
            direct_value: DirectScalar<N2>,
            nested_value: Composite<FixedTailFmt>,
            optional_serde: Option<N2>,
            optional_direct: Option<DirectScalar<N2>>,
            optional_nested: Option<Composite<FixedTailFmt>>,
            optional_direct_inline: Option<DirectScalar<Field<Numeric<2, 2>, Fixed<2>>>>,
            optional_nested_inline: Option<Composite<Frame<Field<Ascii<0, 12>, AsciiLength<2>>, FixedTailFmt>>>,
        }
    }

    crate::delimited_format! {
        struct FieldSyntaxDelimitedFmt for FieldSyntaxRecord, PIPE_SEPARATOR {
            serde_value: N2,
            direct_value: DirectScalar<N2>,
            nested_value: Composite<FixedTailFmt>,
            optional_serde: Option<N2>,
            optional_direct: Option<DirectScalar<N2>>,
            optional_nested: Option<Composite<FixedTailFmt>>,
            optional_direct_inline: Option<DirectScalar<Field<Numeric<2, 2>, Fixed<2>>>>,
            optional_nested_inline: Option<Composite<Frame<Field<Ascii<0, 12>, AsciiLength<2>>, FixedTailFmt>>>,
        }
    }

    crate::bitmap_format! {
        struct FieldSyntaxBitmapFmt for FieldSyntaxRecord, crate::bitmap::BitmapLayout::iso(1), BitmapBinaryWord {
            2 => serde_value: N2,
            3 => direct_value: DirectScalar<N2>,
            4 => nested_value: Composite<FixedTailFmt>,
            5 => optional_serde: Option<N2>,
            6 => optional_direct: Option<DirectScalar<N2>>,
            7 => optional_nested: Option<Composite<FixedTailFmt>>,
            8 => optional_direct_inline: Option<DirectScalar<Field<Numeric<2, 2>, Fixed<2>>>>,
            9 => optional_nested_inline: Option<Composite<Frame<Field<Ascii<0, 12>, AsciiLength<2>>, FixedTailFmt>>>,
        }
    }

    crate::ber_tlv_format! {
        struct FieldSyntaxBerTlvFmt for FieldSyntaxRecord {
            "02" => serde_value: N2,
            "03" => direct_value: DirectScalar<N2>,
            "04" => nested_value: Composite<FixedTailFmt>,
            "05" => optional_serde: Option<N2>,
            "06" => optional_direct: Option<DirectScalar<N2>>,
            "07" => optional_nested: Option<Composite<FixedTailFmt>>,
            "08" => optional_direct_inline: Option<DirectScalar<Field<Numeric<2, 2>, Fixed<2>>>>,
            "09" => optional_nested_inline: Option<Composite<Frame<Field<Ascii<0, 12>, AsciiLength<2>>, FixedTailFmt>>>,
        }
    }

    #[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(transparent)]
    struct DualStan(u8);

    impl ScalarValue for DualStan {
        fn encode_with<F: ScalarFmt>(&self, output: &mut &mut [u8], scratch: &mut &mut [u8]) -> Result<(), Error> {
            F::encode_u64(output, scratch, u64::from(self.0) + 10)
        }

        fn decode_with<'de, F: ScalarFmt>(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self, Error> {
            let value = F::decode_u64(input, scratch)?;
            let value = value.checked_sub(10).ok_or(Error::Invalid)?;
            let value = u8::try_from(value).map_err(|_| Error::Invalid)?;
            Ok(Self(value))
        }
    }

    #[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    struct SerdeDualStanData {
        f011_stan: DualStan,
    }

    #[derive(Debug, Default, PartialEq, Eq)]
    struct ManualDualStanData {
        f011_stan: DualStan,
    }

    crate::bitmap_format! {
        struct SerdeDualStanDataFmt for SerdeDualStanData, crate::bitmap::BitmapLayout::iso(1), BitmapBinaryWord {
            11 => f011_stan: N2,
        }
    }

    crate::bitmap_format! {
        struct ManualDualStanDataFmt for ManualDualStanData, crate::bitmap::BitmapLayout::iso(1), BitmapBinaryWord {
            11 => f011_stan: DirectScalar<N2>,
        }
    }

    #[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    struct BoolScalarData {
        value: bool,
    }

    crate::concat_format! {
        struct BoolScalarDataFmt for BoolScalarData {
            value: N2,
        }
    }

    #[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    struct BytesScalarData {
        value: Vec<u8>,
    }

    crate::concat_format! {
        struct BytesScalarDataFmt for BytesScalarData {
            value: A4,
        }
    }

    #[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    struct NestedShape {
        inner: String,
    }

    #[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    struct StructScalarData {
        value: NestedShape,
    }

    crate::concat_format! {
        struct StructScalarDataFmt for StructScalarData {
            value: A4,
        }
    }

    #[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    struct VariantSelector {
        code: String,
    }

    crate::concat_format! {
        struct VariantSelectorFmt for VariantSelector {
            code: A4,
        }
    }

    #[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    struct VariantATail {
        tail: String,
    }

    crate::concat_format! {
        struct VariantATailFmt for VariantATail {
            tail: N2,
        }
    }

    #[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    struct VariantBTail {
        tail: String,
    }

    crate::concat_format! {
        struct VariantBTailFmt for VariantBTail {
            tail: A4,
        }
    }

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    enum VariantData {
        A(VariantATail),
        B(VariantBTail),
    }

    crate::tagged_format! {
        #[doc = "Test format for an internally tagged enum."]
        struct VariantDataFmt for VariantData {
            _: A4 = b"AXAA" => A(VariantATailFmt) if |remaining_len| remaining_len == 2,
            _: A4 = b"AXBB" => B(VariantBTailFmt) if |remaining_len| remaining_len == 4,
        }
    }

    #[derive(Debug, PartialEq, Eq, Serialize)]
    struct BorrowedVariantTail<'a> {
        tail: &'a str,
    }

    crate::concat_format! {
        struct BorrowedVariantTailFmt for<'a> BorrowedVariantTail<'a> {
            tail: A4,
        }
    }

    #[derive(Debug, PartialEq, Eq, Serialize)]
    enum BorrowedVariantData<'a> {
        A(BorrowedVariantTail<'a>),
        B(BorrowedVariantTail<'a>),
    }

    crate::tagged_format! {
        struct BorrowedVariantDataFmt for<'a> BorrowedVariantData<'a> {
            _: A4 = b"AXAA" => A(BorrowedVariantTailFmt),
            _: A4 = b"AXBB" => B(BorrowedVariantTailFmt),
        }
    }

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    enum RetainedVariantData {
        A(VariantATail),
        B(VariantBTail),
    }

    crate::choice_format! {
        #[doc = "Test format for an externally selected enum body."]
        struct RetainedVariantDataFmt for RetainedVariantData, VariantSelector {
            A(VariantATailFmt) if |selector| selector.code == "AXAA",
            B(VariantBTailFmt) if |selector| selector.code == "AXBB",
        }
    }

    crate::choice_format! {
        struct BorrowedRetainedVariantDataFmt for<'a> BorrowedVariantData<'a>, VariantSelector {
            A(BorrowedVariantTailFmt) if |selector| selector.code == "AXAA",
            B(BorrowedVariantTailFmt) if |selector| selector.code == "AXBB",
        }
    }

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct RetainedVariantRecord {
        selector: VariantSelector,
        body: RetainedVariantData,
        suffix: String,
    }

    crate::concat_format! {
        struct RetainedVariantRecordFmt for RetainedVariantRecord {
            selector: Composite<VariantSelectorFmt>,
            body: Composite<RetainedVariantDataFmt>::with(selector),
            suffix: A4,
        }
    }

    crate::delimited_format! {
        struct DelimitedRetainedVariantRecordFmt for RetainedVariantRecord, b'|' {
            selector: Composite<VariantSelectorFmt>,
            body: Composite<RetainedVariantDataFmt>::with(selector),
            suffix: A4,
        }
    }

    #[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    struct DelimitedSlots {
        first: String,
        second: String,
        third: String,
    }

    type DelimitedSecond = Field<Ascii<0, 3>, WireFixed<3>, crate::chain!(crate::PadRight<3, b' '>)>;

    crate::delimited_format! {
        struct DelimitedSlotsFmt for DelimitedSlots, b'\\' {
            first: A4,
            second: DelimitedSecond,
            third: A4,
        }
    }

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct ConcatNoDefault {
        first: String,
        second: String,
    }

    crate::concat_format! {
        struct ConcatNoDefaultFmt for ConcatNoDefault {
            _: A4 = b"HEAD",
            first: N2,
            second: A4,
        }
    }

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct DelimitedNoDefault {
        first: String,
        second: String,
    }

    crate::delimited_format! {
        struct DelimitedNoDefaultFmt for DelimitedNoDefault, b'|' {
            _: A4 = b"HEAD",
            first: N2,
            second: A4,
        }
    }

    type A5Padded = Field<Ascii<0, 5>, WireFixed<5>, crate::chain!(crate::PadRight<5, b' '>)>;
    type CountN2 = Field<Numeric<1, 2>, Fixed<2>, PadLeft<2, b'0'>>;
    type CountedAsciiListFmt =
        Frame<Field<Binary<7, 19>, AsciiLength<2>>, BoundedList<String, AsciiLength<2>, DirectScalar<A5Padded>, Separator<b'/'>, 3>>;
    type ScalarCountedAsciiListFmt =
        Frame<Field<Binary<7, 19>, AsciiLength<2>>, BoundedList<String, Length<CountN2>, DirectScalar<A5Padded>, Separator<b'/'>, 3>>;
    type FixedAsciiListFmt = FixedCountList<String, DirectScalar<A5Padded>, 3>;

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(transparent)]
    struct NoDefaultAbsentValue(String);

    type OptionalNoDefaultAbsentFmt = OptionalAbsent<NoDefaultAbsentValue, SerdeScalar<A4>, ByteFill<b' '>, 4>;
    type RepeatedNoDefaultByteFillFmt =
        FixedAreaList<NoDefaultAbsentValue, EbcdicLength<1>, OptionalAbsent<NoDefaultAbsentValue, SerdeScalar<A4>, ByteFill<b' '>, 4>, 2>;

    crate::absent_format! {
        #[doc = "Test format for an explicit literal absent encoding."]
        struct NoDefaultLiteralAbsentFmt {
            _: A4 = b"NONE",
        }
    }

    type OptionalNoDefaultLiteralAbsentFmt = OptionalAbsent<NoDefaultAbsentValue, SerdeScalar<A4>, NoDefaultLiteralAbsentFmt, 4>;
    type RepeatedNoDefaultAbsentFmt = FixedAreaList<
        NoDefaultAbsentValue,
        EbcdicLength<1>,
        OptionalAbsent<NoDefaultAbsentValue, SerdeScalar<A4>, NoDefaultLiteralAbsentFmt, 4>,
        2,
    >;

    #[test]
    fn test_bounded_list_roundtrip_and_edges() {
        let value = vec!["ABC".to_owned(), "DEF".to_owned(), "GHI".to_owned()];
        let mut output = [0u8; 64];
        let mut scratch = [0u8; 64];
        let total = output.len();
        let used = {
            let mut out = output.as_mut_slice();
            CountedAsciiListFmt::encode(&mut out, scratch.as_mut_slice(), &value).unwrap();
            total - out.len()
        };
        assert_eq!(&output[..used], b"1903ABC  /DEF  /GHI  ");

        let mut input = &output[..used];
        let decoded = CountedAsciiListFmt::decode(&mut input, scratch.as_mut_slice()).unwrap();
        assert!(input.is_empty());
        assert_eq!(decoded, value);

        let used = {
            let mut out = output.as_mut_slice();
            ScalarCountedAsciiListFmt::encode(&mut out, scratch.as_mut_slice(), &value).unwrap();
            total - out.len()
        };
        assert_eq!(&output[..used], b"1903ABC  /DEF  /GHI  ");

        let mut input = &output[..used];
        let decoded = ScalarCountedAsciiListFmt::decode(&mut input, scratch.as_mut_slice()).unwrap();
        assert!(input.is_empty());
        assert_eq!(decoded, value);

        let used = {
            let mut out = output.as_mut_slice();
            FixedAsciiListFmt::encode(&mut out, scratch.as_mut_slice(), &value).unwrap();
            total - out.len()
        };
        assert_eq!(&output[..used], b"ABC  DEF  GHI  ");

        let mut input = &output[..used];
        let decoded = FixedAsciiListFmt::decode(&mut input, scratch.as_mut_slice()).unwrap();
        assert!(input.is_empty());
        assert_eq!(decoded, value);

        let too_many = vec!["A".to_owned(), "B".to_owned(), "C".to_owned(), "D".to_owned()];
        let mut out = output.as_mut_slice();
        assert_eq!(
            error_kind(CountedAsciiListFmt::encode(&mut out, scratch.as_mut_slice(), &too_many)),
            Err(Error::Invalid)
        );
        let mut out = output.as_mut_slice();
        assert_eq!(
            error_kind(FixedAsciiListFmt::encode(&mut out, scratch.as_mut_slice(), &too_many)),
            Err(Error::Invalid)
        );

        let mut invalid = b"1903ABC  XDEF  XGHI  ".as_slice();
        assert_eq!(
            error_kind(CountedAsciiListFmt::decode(&mut invalid, scratch.as_mut_slice())),
            Err(Error::Invalid)
        );
    }

    #[test]
    fn test_delimited_slots_roundtrip_and_edges() {
        let value = DelimitedSlots {
            first: "ABCD".into(),
            second: "X".into(),
            third: "WXYZ".into(),
        };
        let mut output = [0u8; 32];
        let mut scratch = [0u8; 32];
        let total = output.len();
        let used = {
            let mut out = output.as_mut_slice();
            DelimitedSlotsFmt::encode(&mut out, scratch.as_mut_slice(), &value).unwrap();
            total - out.len()
        };
        assert_eq!(&output[..used], b"ABCD\\X  \\WXYZ");

        let mut input = &output[..used];
        let decoded = DelimitedSlotsFmt::decode(&mut input, scratch.as_mut_slice()).unwrap();
        assert!(input.is_empty());
        assert_eq!(decoded, value);

        let invalid = DelimitedSlots {
            first: "AB\\D".into(),
            ..value
        };
        let mut out = output.as_mut_slice();
        assert_eq!(
            error_kind(DelimitedSlotsFmt::encode(&mut out, scratch.as_mut_slice(), &invalid)),
            Err(Error::Invalid)
        );

        let mut invalid = b"ABCD\\X  ".as_slice();
        assert_eq!(
            error_kind(DelimitedSlotsFmt::decode(&mut invalid, scratch.as_mut_slice())),
            Err(Error::Invalid)
        );
    }

    #[test]
    fn test_concat_and_delimited_decode_without_default() {
        let concat = ConcatNoDefault {
            first: "12".into(),
            second: "ABCD".into(),
        };
        let delimited = DelimitedNoDefault {
            first: "12".into(),
            second: "ABCD".into(),
        };
        let mut output = [0u8; 32];
        let mut scratch = [0u8; 32];

        let concat_used = {
            let total = output.len();
            let mut out = output.as_mut_slice();
            ConcatNoDefaultFmt::encode(&mut out, scratch.as_mut_slice(), &concat).unwrap();
            total - out.len()
        };
        assert_eq!(&output[..concat_used], b"HEAD12ABCD");
        let mut input = &output[..concat_used];
        assert_eq!(ConcatNoDefaultFmt::decode(&mut input, scratch.as_mut_slice()).unwrap(), concat);
        assert!(input.is_empty());

        let delimited_used = {
            let total = output.len();
            let mut out = output.as_mut_slice();
            DelimitedNoDefaultFmt::encode(&mut out, scratch.as_mut_slice(), &delimited).unwrap();
            total - out.len()
        };
        assert_eq!(&output[..delimited_used], b"HEAD|12|ABCD");
        let mut input = &output[..delimited_used];
        assert_eq!(
            DelimitedNoDefaultFmt::decode(&mut input, scratch.as_mut_slice()).unwrap(),
            delimited
        );
        assert!(input.is_empty());
    }

    #[test]
    fn test_absent_wrappers_without_default() {
        let mut output = [0u8; 32];
        let mut scratch = [0u8; 32];

        let optional_none = {
            let total = output.len();
            let mut out = output.as_mut_slice();
            OptionalNoDefaultAbsentFmt::encode(&mut out, scratch.as_mut_slice(), &None).unwrap();
            total - out.len()
        };
        assert_eq!(&output[..optional_none], b"    ");
        let mut input = &output[..optional_none];
        assert_eq!(
            OptionalNoDefaultAbsentFmt::decode(&mut input, scratch.as_mut_slice()).unwrap(),
            None
        );
        assert!(input.is_empty());

        let optional_some = Some(NoDefaultAbsentValue("ABCD".into()));
        let optional_some_used = {
            let total = output.len();
            let mut out = output.as_mut_slice();
            OptionalNoDefaultAbsentFmt::encode(&mut out, scratch.as_mut_slice(), &optional_some).unwrap();
            total - out.len()
        };
        assert_eq!(&output[..optional_some_used], b"ABCD");
        let mut input = &output[..optional_some_used];
        assert_eq!(
            OptionalNoDefaultAbsentFmt::decode(&mut input, scratch.as_mut_slice()).unwrap(),
            optional_some
        );
        assert!(input.is_empty());

        let values = vec![NoDefaultAbsentValue("ABCD".into())];
        let repeated_fill_used = {
            let total = output.len();
            let mut out = output.as_mut_slice();
            RepeatedNoDefaultByteFillFmt::encode(&mut out, scratch.as_mut_slice(), &values).unwrap();
            total - out.len()
        };
        assert_eq!(&output[..repeated_fill_used], b"\xF4ABCD    ");
        let mut input = &output[..repeated_fill_used];
        assert_eq!(
            RepeatedNoDefaultByteFillFmt::decode(&mut input, scratch.as_mut_slice()).unwrap(),
            values
        );
        assert!(input.is_empty());

        let repeated_absent_used = {
            let total = output.len();
            let mut out = output.as_mut_slice();
            RepeatedNoDefaultAbsentFmt::encode(&mut out, scratch.as_mut_slice(), &values).unwrap();
            total - out.len()
        };
        assert_eq!(&output[..repeated_absent_used], b"\xF4ABCDNONE");
        let mut input = &output[..repeated_absent_used];
        assert_eq!(
            RepeatedNoDefaultAbsentFmt::decode(&mut input, scratch.as_mut_slice()).unwrap(),
            values
        );
        assert!(input.is_empty());

        let literal_none = {
            let total = output.len();
            let mut out = output.as_mut_slice();
            OptionalNoDefaultLiteralAbsentFmt::encode(&mut out, scratch.as_mut_slice(), &None).unwrap();
            total - out.len()
        };
        assert_eq!(&output[..literal_none], b"NONE");
        let mut input = &output[..literal_none];
        assert_eq!(
            OptionalNoDefaultLiteralAbsentFmt::decode(&mut input, scratch.as_mut_slice()).unwrap(),
            None
        );
        assert!(input.is_empty());
    }

    #[test]
    fn test_struct_error_field_paths() {
        let mut output = [0u8; 32];
        let mut scratch = [0u8; 32];

        let invalid = DelimitedSlots {
            first: "AB\\D".into(),
            second: "X".into(),
            third: "WXYZ".into(),
        };
        let mut out = output.as_mut_slice();
        let error = DelimitedSlotsFmt::encode(&mut out, scratch.as_mut_slice(), &invalid).unwrap_err();
        assert_eq!(error.kind, Error::Invalid);
        assert_eq!(error.path(), ["first"]);
        assert!(!error.truncated);

        let nested = NestedConcat {
            head: "12".into(),
            inner: Some(FixedTail {
                a: "123456".into(),
                b: "78".into(),
                c: "ABC".into(),
            }),
            tail: "34".into(),
        };
        let mut out = output.as_mut_slice();
        let error = NestedConcatFmt::encode(&mut out, scratch.as_mut_slice(), &nested).unwrap_err();
        assert_eq!(error.kind, Error::InvalidValueLength);
        assert_eq!(error.path(), ["inner", "c"]);
        assert!(!error.truncated);
    }

    #[test]
    fn test_concat_roundtrip() {
        for (value, expected) in [
            (
                NestedConcat {
                    head: "12".into(),
                    inner: Some(FixedTail {
                        a: "123456".into(),
                        b: "78".into(),
                        c: "ABCD".into(),
                    }),
                    tail: "34".into(),
                },
                Some(&b"123412345678ABCD"[..]),
            ),
            (
                NestedConcat {
                    head: "12".into(),
                    inner: None,
                    tail: "34".into(),
                },
                Some(&b"1234"[..]),
            ),
        ] {
            let mut output = [0u8; 32];
            let mut scratch = [0u8; 32];
            let total = output.len();
            let encoded = {
                let mut out_ptr = output.as_mut_slice();
                NestedConcatFmt::encode(&mut out_ptr, scratch.as_mut_slice(), &value).map(|_| total - out_ptr.len())
            };
            if let Some(expected) = expected {
                let encoded = encoded.unwrap();
                assert_eq!(&output[..encoded], expected);
                let mut input = &output[..encoded];
                let mut decode_scratch = [0u8; 32];
                let decoded = NestedConcatFmt::decode(&mut input, decode_scratch.as_mut_slice()).unwrap();
                assert_eq!(decoded, value);
                assert!(input.is_empty());
            } else {
                assert_eq!(error_kind(encoded), Err(Error::Invalid));
            }
        }
    }

    #[test]
    fn test_borrowed_struct_decode() {
        fn roundtrip<T, F>(value: &T, output: &mut [u8], scratch: &mut [u8]) -> usize
        where
            F: CompositeFmt<T>,
        {
            let total = output.len();
            let mut out = output;
            F::encode(&mut out, scratch, value).map(|_| total - out.len()).unwrap()
        }

        let concat = BorrowedConcat {
            ascii: "ABCD",
            ebcdic: "WXYZ",
        };
        let delimited = BorrowedDelimited {
            ascii: "ABCD",
            ebcdic: "WXYZ",
        };
        let bitmap = BorrowedBitmap {
            required: "ABCD",
            optional: None,
            ebcdic: Some("WXYZ"),
        };
        let tlv = BorrowedTlv {
            ascii: "ABCD",
            ebcdic: "WXYZ",
            tail: Some(BorrowedConcat {
                ascii: "1234",
                ebcdic: "QRST",
            }),
        };

        let mut output = [0u8; 96];
        let mut scratch = [0u8; 96];

        let used = roundtrip::<_, BorrowedConcatFmt>(&concat, output.as_mut_slice(), scratch.as_mut_slice());
        let mut input = &output[..used];
        let mut decode_scratch = [0u8; 96];
        let decoded = BorrowedConcatFmt::decode(&mut input, decode_scratch.as_mut_slice()).unwrap();
        assert_eq!(decoded, concat);
        assert!(input.is_empty());

        let used = roundtrip::<_, BorrowedDelimitedFmt>(&delimited, output.as_mut_slice(), scratch.as_mut_slice());
        let mut input = &output[..used];
        let decoded = BorrowedDelimitedFmt::decode(&mut input, decode_scratch.as_mut_slice()).unwrap();
        assert_eq!(decoded, delimited);
        assert!(input.is_empty());

        let used = roundtrip::<_, BorrowedBitmapFmt>(&bitmap, output.as_mut_slice(), scratch.as_mut_slice());
        let mut input = &output[..used];
        let decoded = BorrowedBitmapFmt::decode(&mut input, decode_scratch.as_mut_slice()).unwrap();
        assert_eq!(decoded, bitmap);
        assert!(input.is_empty());

        let used = roundtrip::<_, BorrowedTlvFmt>(&tlv, output.as_mut_slice(), scratch.as_mut_slice());
        let mut input = &output[..used];
        let decoded = BorrowedTlvFmt::decode(&mut input, decode_scratch.as_mut_slice()).unwrap();
        assert_eq!(decoded, tlv);
        assert!(input.is_empty());
    }

    #[test]
    fn test_framed_concat_roundtrip() {
        let value = FramedConcat {
            head: "12".into(),
            inner: FixedTail {
                a: "123456".into(),
                b: "78".into(),
                c: "ABCD".into(),
            },
            tail: "34".into(),
        };
        let mut output = [0u8; 32];
        let mut scratch = [0u8; 64];
        let total = output.len();
        let encoded = {
            let mut out_ptr = output.as_mut_slice();
            FramedConcatFmt::encode(&mut out_ptr, scratch.as_mut_slice(), &value).map(|_| total - out_ptr.len())
        }
        .unwrap();
        assert_eq!(&output[..encoded], b"121212345678ABCD34");
        let mut input = &output[..encoded];
        let mut decode_scratch = [0u8; 64];
        let decoded = FramedConcatFmt::decode(&mut input, decode_scratch.as_mut_slice()).unwrap();
        assert_eq!(decoded, value);
        assert!(input.is_empty());
    }

    #[test]
    fn test_framed_hex_concat_roundtrip() {
        let value = FramedHexConcat {
            head: "12".into(),
            tail: "34".into(),
            inner: FixedTail {
                a: "123456".into(),
                b: "78".into(),
                c: "ABCD".into(),
            },
        };
        let mut output = [0u8; 64];
        let mut scratch = [0u8; 64];
        let total = output.len();
        let encoded = {
            let mut out_ptr = output.as_mut_slice();
            FramedHexConcatFmt::encode(&mut out_ptr, scratch.as_mut_slice(), &value).map(|_| total - out_ptr.len())
        }
        .unwrap();
        assert_eq!(&output[..encoded], b"123424313233343536373841424344");
        let mut input = &output[..encoded];
        let mut decode_scratch = [0u8; 64];
        let decoded = FramedHexConcatFmt::decode(&mut input, decode_scratch.as_mut_slice()).unwrap();
        assert_eq!(decoded, value);
        assert!(input.is_empty());
    }

    #[test]
    fn test_trailing_length_frame_roundtrip_and_validation() {
        let mut output = [0u8; 32];
        let mut scratch = [0u8; 64];
        for (value, expected) in [
            (
                TrailingLengthData {
                    base: "AB".into(),
                    tail1: None,
                    tail2: None,
                },
                &b"02AB      "[..],
            ),
            (
                TrailingLengthData {
                    base: "AB".into(),
                    tail1: Some("CDE".into()),
                    tail2: None,
                },
                &b"05ABCDE   "[..],
            ),
            (
                TrailingLengthData {
                    base: "AB".into(),
                    tail1: None,
                    tail2: Some("XYZ".into()),
                },
                &b"08AB   XYZ"[..],
            ),
        ] {
            let total = output.len();
            let encoded = {
                let mut out = output.as_mut_slice();
                TrailingLengthDataFmt::encode(&mut out, scratch.as_mut_slice(), &value).map(|_| total - out.len())
            }
            .unwrap();
            assert_eq!(&output[..encoded], expected);

            let mut input = &output[..encoded];
            let decoded = TrailingLengthDataFmt::decode(&mut input, scratch.as_mut_slice()).unwrap();
            assert_eq!(decoded, value);
            assert!(input.is_empty());
        }

        for invalid in [&b"03AB      "[..], &b"02ABCDE   "[..]] {
            let mut input = invalid;
            assert_eq!(
                error_kind(TrailingLengthDataFmt::decode(&mut input, scratch.as_mut_slice())),
                Err(Error::Invalid)
            );
        }
    }

    #[test]
    fn test_local_bitmap_roundtrip() {
        let value = LocalBitmapData {
            a: Some("12".into()),
            b: Some("ABCD".into()),
        };
        let mut output = [0u8; 32];
        let mut scratch = [0u8; 32];
        let total = output.len();
        let encoded = {
            let mut out_ptr = output.as_mut_slice();
            LocalBitmapDataFmt::encode(&mut out_ptr, scratch.as_mut_slice(), &value).map(|_| total - out_ptr.len())
        }
        .unwrap();
        assert_eq!(&output[..encoded], b"HEAD\x60\x00\x00\x0012ABCD");
        let mut input = &output[..encoded];
        let mut decode_scratch = [0u8; 32];
        let decoded = LocalBitmapDataFmt::decode(&mut input, decode_scratch.as_mut_slice()).unwrap();
        assert_eq!(decoded, value);
        assert!(input.is_empty());
    }

    #[test]
    fn test_bitmap_decode_without_default() {
        let values = [
            BitmapNoDefault {
                head_code: "12".into(),
                f002_required: "34".into(),
                f003_optional: Some("ABCD".into()),
            },
            BitmapNoDefault {
                head_code: "12".into(),
                f002_required: "34".into(),
                f003_optional: None,
            },
        ];

        for value in values {
            let mut output = [0u8; 32];
            let mut scratch = [0u8; 32];
            let total = output.len();
            let used = {
                let mut out_ptr = output.as_mut_slice();
                BitmapNoDefaultFmt::encode(&mut out_ptr, scratch.as_mut_slice(), &value).map(|_| total - out_ptr.len())
            }
            .unwrap();
            let mut input = &output[..used];
            let decoded = BitmapNoDefaultFmt::decode(&mut input, scratch.as_mut_slice()).unwrap();
            assert_eq!(decoded, value);
            assert!(input.is_empty());
        }
    }

    #[test]
    fn test_serde_scalar_field_values_roundtrip() {
        for (value, expected) in [
            (
                SerdeScalarBitmapData {
                    f003_processing_code: ProcessingCode("123456".into()),
                    f011_stan: Some(Stan(654321)),
                },
                &b"\x20\x20\x00\x00\x00\x00\x00\x00123456654321"[..],
            ),
            (
                SerdeScalarBitmapData {
                    f003_processing_code: ProcessingCode("123456".into()),
                    f011_stan: None,
                },
                &b"\x20\x00\x00\x00\x00\x00\x00\x00123456"[..],
            ),
        ] {
            let mut output = [0u8; 32];
            let mut scratch = [0u8; 32];
            let total = output.len();
            let encoded = {
                let mut out_ptr = output.as_mut_slice();
                SerdeScalarBitmapDataFmt::encode(&mut out_ptr, scratch.as_mut_slice(), &value).map(|_| total - out_ptr.len())
            }
            .unwrap();
            assert_eq!(&output[..encoded], expected);

            let mut input = &output[..encoded];
            let mut decode_scratch = [0u8; 32];
            let decoded = SerdeScalarBitmapDataFmt::decode(&mut input, decode_scratch.as_mut_slice()).unwrap();
            assert_eq!(decoded, value);
            assert!(input.is_empty());
        }
    }

    #[test]
    fn test_manual_scalar_value_escape_hatch_roundtrip() {
        for (value, expected) in [
            (
                ManualScalarBitmapData {
                    f003_processing_code: ManualProcessingCode("123456".into()),
                    f011_stan: Some(ManualStan(654_321)),
                },
                &b"\x20\x20\x00\x00\x00\x00\x00\x00123456654321"[..],
            ),
            (
                ManualScalarBitmapData {
                    f003_processing_code: ManualProcessingCode("123456".into()),
                    f011_stan: None,
                },
                &b"\x20\x00\x00\x00\x00\x00\x00\x00123456"[..],
            ),
        ] {
            let mut output = [0u8; 32];
            let mut scratch = [0u8; 32];
            let total = output.len();
            let encoded = {
                let mut out_ptr = output.as_mut_slice();
                ManualScalarBitmapDataFmt::encode(&mut out_ptr, scratch.as_mut_slice(), &value).map(|_| total - out_ptr.len())
            }
            .unwrap();
            assert_eq!(&output[..encoded], expected);

            let mut input = &output[..encoded];
            let mut decode_scratch = [0u8; 32];
            let decoded = ManualScalarBitmapDataFmt::decode(&mut input, decode_scratch.as_mut_slice()).unwrap();
            assert_eq!(decoded, value);
            assert!(input.is_empty());
        }
    }

    fn assert_field_syntax_roundtrip<F>(value: &FieldSyntaxRecord)
    where
        F: CompositeFmt<FieldSyntaxRecord>,
        for<'de> F::Decoded<'de>: core::fmt::Debug + PartialEq<FieldSyntaxRecord>,
    {
        let mut output = [0u8; 128];
        let mut scratch = [0u8; 128];
        let total = output.len();
        let used = {
            let mut out_ptr = output.as_mut_slice();
            F::encode(&mut out_ptr, scratch.as_mut_slice(), value).map(|_| total - out_ptr.len())
        }
        .unwrap();
        let mut input = &output[..used];
        let mut decode_scratch = [0u8; 128];
        let decoded = F::decode(&mut input, decode_scratch.as_mut_slice()).unwrap();
        assert_eq!(&decoded, value);
        assert!(input.is_empty());
    }

    #[test]
    fn test_record_field_syntax_roundtrip() {
        let full = FieldSyntaxRecord {
            serde_value: Stan(12),
            direct_value: ManualStan(34),
            nested_value: FixedTail {
                a: "123456".into(),
                b: "78".into(),
                c: "ABCD".into(),
            },
            optional_serde: Some(Stan(56)),
            optional_direct: Some(ManualStan(78)),
            optional_nested: Some(FixedTail {
                a: "654321".into(),
                b: "87".into(),
                c: "DCBA".into(),
            }),
            optional_direct_inline: Some(ManualStan(90)),
            optional_nested_inline: Some(FixedTail {
                a: "112233".into(),
                b: "44".into(),
                c: "EFGH".into(),
            }),
        };
        assert_field_syntax_roundtrip::<FieldSyntaxConcatFmt>(&full);
        assert_field_syntax_roundtrip::<FieldSyntaxDelimitedFmt>(&full);
        assert_field_syntax_roundtrip::<FieldSyntaxBitmapFmt>(&full);
        assert_field_syntax_roundtrip::<FieldSyntaxBerTlvFmt>(&full);

        let sparse = FieldSyntaxRecord {
            serde_value: Stan(12),
            direct_value: ManualStan(34),
            nested_value: FixedTail {
                a: "123456".into(),
                b: "78".into(),
                c: "ABCD".into(),
            },
            optional_serde: None,
            optional_direct: None,
            optional_nested: None,
            optional_direct_inline: None,
            optional_nested_inline: None,
        };
        assert_field_syntax_roundtrip::<FieldSyntaxConcatFmt>(&sparse);
        assert_field_syntax_roundtrip::<FieldSyntaxDelimitedFmt>(&sparse);
        assert_field_syntax_roundtrip::<FieldSyntaxBitmapFmt>(&sparse);
        assert_field_syntax_roundtrip::<FieldSyntaxBerTlvFmt>(&sparse);

        let partial_tail = FieldSyntaxRecord {
            optional_serde: Some(Stan(56)),
            ..sparse
        };
        assert_field_syntax_roundtrip::<FieldSyntaxConcatFmt>(&partial_tail);

        let tail_gap = FieldSyntaxRecord {
            serde_value: Stan(12),
            direct_value: ManualStan(34),
            nested_value: FixedTail {
                a: "123456".into(),
                b: "78".into(),
                c: "ABCD".into(),
            },
            optional_serde: None,
            optional_direct: Some(ManualStan(78)),
            optional_nested: None,
            optional_direct_inline: None,
            optional_nested_inline: None,
        };
        assert_eq!(
            error_kind({
                let mut output = [0u8; 128];
                let mut scratch = [0u8; 128];
                let mut out_ptr = output.as_mut_slice();
                FieldSyntaxConcatFmt::encode(&mut out_ptr, scratch.as_mut_slice(), &tail_gap)
            }),
            Err(Error::Invalid)
        );
    }

    #[test]
    fn test_borrowed_scalar_value_roundtrip() {
        let value = "ABCD";
        let mut output = [0u8; 8];
        let mut scratch = [0u8; 8];
        let total = output.len();
        let encoded = {
            let mut out_ptr = output.as_mut_slice();
            <DirectScalar<A4, &str> as CompositeFmt<&str>>::encode(&mut out_ptr, scratch.as_mut_slice(), &value)
                .map(|_| total - out_ptr.len())
        }
        .unwrap();
        assert_eq!(&output[..encoded], b"ABCD");

        let mut input = &output[..encoded];
        let decoded = <DirectScalar<A4, &str> as CompositeFmt<&str>>::decode(&mut input, scratch.as_mut_slice()).unwrap();
        assert_eq!(decoded, value);
        assert!(input.is_empty());
    }

    #[test]
    fn test_same_type_can_use_serde_or_manual_scalar_path() {
        let serde_value = SerdeDualStanData { f011_stan: DualStan(17) };
        let manual_value = ManualDualStanData { f011_stan: DualStan(17) };

        let mut serde_output = [0u8; 16];
        let mut serde_scratch = [0u8; 16];
        let serde_len = {
            let total = serde_output.len();
            let mut out_ptr = serde_output.as_mut_slice();
            SerdeDualStanDataFmt::encode(&mut out_ptr, serde_scratch.as_mut_slice(), &serde_value).map(|_| total - out_ptr.len())
        }
        .unwrap();
        assert_eq!(&serde_output[..serde_len], &b"\x00\x20\x00\x00\x00\x00\x00\x0017"[..]);

        let mut manual_output = [0u8; 16];
        let mut manual_scratch = [0u8; 16];
        let manual_len = {
            let total = manual_output.len();
            let mut out_ptr = manual_output.as_mut_slice();
            ManualDualStanDataFmt::encode(&mut out_ptr, manual_scratch.as_mut_slice(), &manual_value).map(|_| total - out_ptr.len())
        }
        .unwrap();
        assert_eq!(&manual_output[..manual_len], &b"\x00\x20\x00\x00\x00\x00\x00\x0027"[..]);

        let mut serde_input = &serde_output[..serde_len];
        let mut serde_decode_scratch = [0u8; 16];
        let serde_decoded = SerdeDualStanDataFmt::decode(&mut serde_input, serde_decode_scratch.as_mut_slice()).unwrap();
        assert_eq!(serde_decoded, serde_value);
        assert!(serde_input.is_empty());

        let mut manual_input = &manual_output[..manual_len];
        let mut manual_decode_scratch = [0u8; 16];
        let manual_decoded = ManualDualStanDataFmt::decode(&mut manual_input, manual_decode_scratch.as_mut_slice()).unwrap();
        assert_eq!(manual_decoded, manual_value);
        assert!(manual_input.is_empty());
    }

    #[test]
    fn test_unsupported_serde_scalar_shapes_return_invalid_format() {
        let mut output = [0u8; 32];
        let mut scratch = [0u8; 32];

        for encode in [
            error_kind({
                let mut out_ptr = output.as_mut_slice();
                BoolScalarDataFmt::encode(&mut out_ptr, scratch.as_mut_slice(), &BoolScalarData { value: true })
            }),
            error_kind({
                let mut out_ptr = output.as_mut_slice();
                BytesScalarDataFmt::encode(&mut out_ptr, scratch.as_mut_slice(), &BytesScalarData { value: vec![0x31, 0x32] })
            }),
            error_kind({
                let mut out_ptr = output.as_mut_slice();
                StructScalarDataFmt::encode(
                    &mut out_ptr,
                    scratch.as_mut_slice(),
                    &StructScalarData {
                        value: NestedShape { inner: "12".into() },
                    },
                )
            }),
        ] {
            assert_eq!(encode, Err(Error::Internal));
        }

        for decode in [
            error_kind(BoolScalarDataFmt::decode(&mut b"12".as_slice(), scratch.as_mut_slice())).map(|_| ()),
            error_kind(BytesScalarDataFmt::decode(&mut b"ABCD".as_slice(), scratch.as_mut_slice())).map(|_| ()),
            error_kind(StructScalarDataFmt::decode(&mut b"ABCD".as_slice(), scratch.as_mut_slice())).map(|_| ()),
        ] {
            assert_eq!(decode, Err(Error::Internal));
        }
    }

    #[test]
    fn test_tagged_format_roundtrip() {
        for (value, expected) in [
            (VariantData::A(VariantATail { tail: "12".into() }), &b"AXAA12"[..]),
            (VariantData::B(VariantBTail { tail: "WXYZ".into() }), &b"AXBBWXYZ"[..]),
        ] {
            let mut output = [0u8; 32];
            let mut scratch = [0u8; 32];
            let total = output.len();
            let encoded = {
                let mut out_ptr = output.as_mut_slice();
                VariantDataFmt::encode(&mut out_ptr, scratch.as_mut_slice(), &value).map(|_| total - out_ptr.len())
            }
            .unwrap();
            assert_eq!(&output[..encoded], expected);
            let mut input = &output[..encoded];
            let mut decode_scratch = [0u8; 32];
            let decoded = VariantDataFmt::decode(&mut input, decode_scratch.as_mut_slice()).unwrap();
            assert_eq!(decoded, value);
            assert!(input.is_empty());
        }
    }

    #[test]
    fn test_retained_selector_choice_roundtrip_and_mismatch() {
        let value = RetainedVariantRecord {
            selector: VariantSelector { code: "AXBB".into() },
            body: RetainedVariantData::B(VariantBTail { tail: "WXYZ".into() }),
            suffix: "DONE".into(),
        };
        let mut output = [0u8; 32];
        let mut scratch = [0u8; 32];
        let total = output.len();
        let used = {
            let mut out = output.as_mut_slice();
            RetainedVariantRecordFmt::encode(&mut out, scratch.as_mut_slice(), &value).map(|_| total - out.len())
        }
        .unwrap();
        assert_eq!(&output[..used], b"AXBBWXYZDONE");

        let mut input = &output[..used];
        let mut decode_scratch = [0u8; 32];
        let decoded = RetainedVariantRecordFmt::decode(&mut input, decode_scratch.as_mut_slice()).unwrap();
        assert_eq!(decoded, value);
        assert!(input.is_empty());

        let used = {
            let mut out = output.as_mut_slice();
            DelimitedRetainedVariantRecordFmt::encode(&mut out, scratch.as_mut_slice(), &value).map(|_| total - out.len())
        }
        .unwrap();
        assert_eq!(&output[..used], b"AXBB|WXYZ|DONE");

        let mut input = &output[..used];
        let decoded = DelimitedRetainedVariantRecordFmt::decode(&mut input, decode_scratch.as_mut_slice()).unwrap();
        assert_eq!(decoded, value);
        assert!(input.is_empty());

        let selector = VariantSelector { code: "AXBB".into() };
        let borrowed = BorrowedVariantData::B(BorrowedVariantTail { tail: "WXYZ" });
        let used = {
            let mut out = output.as_mut_slice();
            let mut scratch_ptr = scratch.as_mut_slice();
            BorrowedRetainedVariantDataFmt::encode_with(&mut out, &mut scratch_ptr, &selector, &borrowed).map(|_| total - out.len())
        }
        .unwrap();
        assert_eq!(&output[..used], b"WXYZ");

        let mut input = &output[..used];
        let mut scratch_ptr = decode_scratch.as_mut_slice();
        let decoded = BorrowedRetainedVariantDataFmt::decode_with(&mut input, &mut scratch_ptr, &selector).unwrap();
        assert_eq!(decoded, borrowed);
        assert!(input.is_empty());

        let mismatch = RetainedVariantRecord {
            selector: VariantSelector { code: "AXAA".into() },
            body: RetainedVariantData::B(VariantBTail { tail: "WXYZ".into() }),
            suffix: "DONE".into(),
        };
        let mut out = output.as_mut_slice();
        assert_eq!(
            error_kind(RetainedVariantRecordFmt::encode(&mut out, scratch.as_mut_slice(), &mismatch)),
            Err(Error::Invalid)
        );

        let mut unknown = &b"AXZZWXYZDONE"[..];
        assert_eq!(
            error_kind(RetainedVariantRecordFmt::decode(&mut unknown, scratch.as_mut_slice())),
            Err(Error::Invalid)
        );

        let with_separator = RetainedVariantRecord {
            selector: VariantSelector { code: "AXBB".into() },
            body: RetainedVariantData::B(VariantBTail { tail: "WX|Z".into() }),
            suffix: "DONE".into(),
        };
        let mut out = output.as_mut_slice();
        assert_eq!(
            error_kind(DelimitedRetainedVariantRecordFmt::encode(
                &mut out,
                scratch.as_mut_slice(),
                &with_separator
            )),
            Err(Error::Invalid)
        );
    }

    #[test]
    fn test_borrowed_tagged_format_roundtrip() {
        let mut output = [0u8; 32];
        let mut scratch = [0u8; 32];

        let value = BorrowedVariantData::B(BorrowedVariantTail { tail: "WXYZ" });
        let used = {
            let total = output.len();
            let mut out = output.as_mut_slice();
            BorrowedVariantDataFmt::encode(&mut out, scratch.as_mut_slice(), &value).map(|_| total - out.len())
        }
        .unwrap();
        assert_eq!(&output[..used], b"AXBBWXYZ");
        let mut input = &output[..used];
        let decoded = BorrowedVariantDataFmt::decode(&mut input, scratch.as_mut_slice()).unwrap();
        assert_eq!(decoded, value);
        assert!(input.is_empty());
    }

    #[test]
    fn test_iso_bitmap_roundtrip() {
        for (value, bitmap) in [
            (
                Auth1100 {
                    f003_processing_code: "123456".into(),
                    f011_stan: "654321".into(),
                    f035_track2_data: Some("1234567890123456=78".into()),
                    f048_fixed_tail: Some(FixedTail {
                        a: "333333".into(),
                        b: "44".into(),
                        c: "WXYZ".into(),
                    }),
                    f097_amount_net_settlement: Some(-12345),
                },
                &b"\xA0\x20\x00\x00\x20\x01\x00\x00\x00\x00\x00\x00\x80\x00\x00\x00"[..],
            ),
            (
                Auth1100 {
                    f003_processing_code: "123456".into(),
                    f011_stan: "654321".into(),
                    f035_track2_data: Some("1234567890123456=78".into()),
                    f048_fixed_tail: None,
                    f097_amount_net_settlement: Some(-12345),
                },
                &b"\xA0\x20\x00\x00\x20\x00\x00\x00\x00\x00\x00\x00\x80\x00\x00\x00"[..],
            ),
        ] {
            let mut output = [0u8; 128];
            let mut scratch = [0u8; 128];
            let total = output.len();
            let used = {
                let mut out_ptr = output.as_mut_slice();
                Auth1100Fmt::encode(&mut out_ptr, scratch.as_mut_slice(), &value).unwrap();
                total - out_ptr.len()
            };

            assert_eq!(&output[..16], bitmap);
            assert_eq!(&output[16..22], b"123456");
            assert_eq!(&output[22..28], b"654321");
            assert_eq!(&output[28..40], b"\xF1\xF0\x12\x34\x56\x78\x90\x12\x34\x56\xD7\x8F");
            if value.f048_fixed_tail.is_some() {
                assert_eq!(&output[40..52], b"33333344WXYZ");
                assert_eq!(&output[52..61], b"D\x00\x00\x00\x00\x00\x01\x23\x45");
            } else {
                assert_eq!(&output[40..49], b"D\x00\x00\x00\x00\x00\x01\x23\x45");
            }

            let mut input = &output[..used];
            let mut decode_scratch = [0u8; 128];
            let decoded = Auth1100Fmt::decode(&mut input, decode_scratch.as_mut_slice()).unwrap();
            assert_eq!(decoded, value);
            assert!(input.is_empty());
        }
    }

    #[test]
    fn test_ber_tlv_roundtrip_and_reject_unknown() {
        let value = TlvData {
            t59_code: "ABCD".into(),
            tdf23_tail: Some(FixedTail {
                a: "123456".into(),
                b: "78".into(),
                c: "WXYZ".into(),
            }),
        };

        let mut output = [0u8; 64];
        let mut scratch = [0u8; 64];
        let total = output.len();
        let used = {
            let mut out_ptr = output.as_mut_slice();
            TlvDataFmt::encode(&mut out_ptr, scratch.as_mut_slice(), &value).unwrap();
            total - out_ptr.len()
        };

        assert_eq!(&output[..used], b"\x59\x04ABCD\xDF\x23\x0C12345678WXYZ");

        let with_unknown = b"\x59\x04ABCD\x9F\x01\x01\xFF\xDF\x23\x0C12345678WXYZ";
        let mut input = &with_unknown[..];
        let mut decode_scratch = [0u8; 64];
        assert_eq!(
            error_kind(TlvDataFmt::decode(&mut input, decode_scratch.as_mut_slice())),
            Err(Error::Invalid)
        );
    }

    #[test]
    fn test_ber_tlv_duplicate_rejected_and_nested_bitmap_roundtrip() {
        let mut dup = &b"\x59\x04ABCD\x59\x04WXYZ"[..];
        let mut scratch = [0u8; 32];
        assert_eq!(
            error_kind(TlvDataFmt::decode(&mut dup, scratch.as_mut_slice())),
            Err(Error::Invalid)
        );

        let value = WithBitmapTlv {
            f003_processing_code: "123456".into(),
            f048_details: Some(TlvData {
                t59_code: "ABCD".into(),
                tdf23_tail: Some(FixedTail {
                    a: "123456".into(),
                    b: "78".into(),
                    c: "WXYZ".into(),
                }),
            }),
        };

        let mut output = [0u8; 128];
        let mut encode_scratch = [0u8; 128];
        let total = output.len();
        let used = {
            let mut out_ptr = output.as_mut_slice();
            WithBitmapTlvFmt::encode(&mut out_ptr, encode_scratch.as_mut_slice(), &value).unwrap();
            total - out_ptr.len()
        };

        assert_eq!(&output[..8], b"\x20\x00\x00\x00\x00\x01\x00\x00");
        assert_eq!(&output[8..14], b"123456");
        assert_eq!(&output[14..used], b"\x59\x04ABCD\xDF\x23\x0C12345678WXYZ");

        let mut input = &output[..used];
        let mut decode_scratch = [0u8; 128];
        let decoded = WithBitmapTlvFmt::decode(&mut input, decode_scratch.as_mut_slice()).unwrap();
        assert_eq!(decoded, value);
        assert!(input.is_empty());
    }

    #[test]
    fn test_ber_tlv_extras_roundtrip_and_flattened_json() {
        let bytes = b"\x59\x04ABCD\x9F\x02\x02\x12\x34";
        let mut input = &bytes[..];
        let mut scratch = [0u8; 64];
        let decoded = TlvWithExtrasFmt::decode(&mut input, scratch.as_mut_slice()).unwrap();
        assert!(input.is_empty());
        assert_eq!(decoded.t59_code, "ABCD");
        assert_eq!(decoded.extras.get("t9F02_unknown").map(String::as_str), Some("1234"));
        let json = serde_json::to_value(&decoded).unwrap();
        assert_eq!(json["t59_code"], "ABCD");
        assert_eq!(json["t9F02_unknown"], "1234");

        let mut output = [0u8; 64];
        let total = output.len();
        let used = {
            let mut out_ptr = output.as_mut_slice();
            TlvWithExtrasFmt::encode(&mut out_ptr, scratch.as_mut_slice(), &decoded).unwrap();
            total - out_ptr.len()
        };
        assert_eq!(&output[..used], bytes);
    }

    #[test]
    fn test_ber_tlv_extras_duplicate_unknown_rejected_for_map() {
        let bytes = b"\x9F\x02\x01\x01\x9F\x02\x01\x02";
        let mut input = &bytes[..];
        let mut scratch = [0u8; 64];
        assert_eq!(
            error_kind(TlvWithExtrasFmt::decode(&mut input, scratch.as_mut_slice())),
            Err(Error::Invalid)
        );
    }

    #[test]
    fn test_ber_tlv_extras_invalid_key_or_value_rejected_on_encode() {
        let invalid_key = TlvWithExtras {
            t59_code: "ABCD".into(),
            extras: BTreeMap::from([("bad".to_owned(), "1234".to_owned())]),
        };
        let invalid_value = TlvWithExtras {
            t59_code: "ABCD".into(),
            extras: BTreeMap::from([("t9F02_unknown".to_owned(), "12fg".to_owned())]),
        };
        let mut output = [0u8; 64];
        let mut scratch = [0u8; 64];
        let mut out_ptr = output.as_mut_slice();
        assert_eq!(
            error_kind(TlvWithExtrasFmt::encode(&mut out_ptr, scratch.as_mut_slice(), &invalid_key)),
            Err(Error::Invalid)
        );
        let mut out_ptr = output.as_mut_slice();
        assert_eq!(
            error_kind(TlvWithExtrasFmt::encode(&mut out_ptr, scratch.as_mut_slice(), &invalid_value)),
            Err(Error::Invalid)
        );
    }

    #[test]
    fn test_ber_tlv_list_roundtrip_preserves_order_and_duplicates() {
        type TlvListFmt = BerTlvList<Vec<(String, String)>>;

        let value = vec![
            ("t59_unknown".to_owned(), "ABCD".to_owned()),
            ("t9F02_unknown".to_owned(), "1234".to_owned()),
            ("t59_unknown".to_owned(), "00FF".to_owned()),
        ];
        let mut output = [0u8; 64];
        let mut scratch = [0u8; 64];
        let used = {
            let total = output.len();
            let mut out = output.as_mut_slice();
            TlvListFmt::encode(&mut out, scratch.as_mut_slice(), &value).map(|_| total - out.len())
        }
        .unwrap();

        let mut input = &output[..used];
        let decoded = TlvListFmt::decode(&mut input, scratch.as_mut_slice()).unwrap();
        assert_eq!(decoded, value);
        assert!(input.is_empty());
    }

    #[test]
    fn test_ber_tlv_list_supports_map_and_newtype_wrappers() {
        #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(transparent)]
        struct TlvSeqWrapper(Vec<(String, String)>);

        type TlvMapFmt = BerTlvList<BTreeMap<String, String>>;
        type TlvSeqWrapperFmt = BerTlvList<TlvSeqWrapper>;

        let map = BTreeMap::from([
            ("t59_unknown".to_owned(), "ABCD".to_owned()),
            ("t9F02_unknown".to_owned(), "1234".to_owned()),
        ]);
        let wrapper = TlvSeqWrapper(vec![
            ("t59_unknown".to_owned(), "ABCD".to_owned()),
            ("t9F02_unknown".to_owned(), "1234".to_owned()),
            ("t59_unknown".to_owned(), "00FF".to_owned()),
        ]);
        let map_bytes = b"\x59\x02\xAB\xCD\x9F\x02\x02\x12\x34";
        let wrapper_bytes = b"\x59\x02\xAB\xCD\x9F\x02\x02\x12\x34\x59\x02\x00\xFF";
        let mut scratch = [0u8; 64];

        let mut output = [0u8; 64];
        let used = {
            let total = output.len();
            let mut out = output.as_mut_slice();
            TlvMapFmt::encode(&mut out, scratch.as_mut_slice(), &map).map(|_| total - out.len())
        }
        .unwrap();
        assert_eq!(&output[..used], map_bytes);
        let mut input = map_bytes.as_slice();
        let decoded = TlvMapFmt::decode(&mut input, scratch.as_mut_slice()).unwrap();
        assert_eq!(decoded, map);
        assert!(input.is_empty());

        let used = {
            let total = output.len();
            let mut out = output.as_mut_slice();
            TlvSeqWrapperFmt::encode(&mut out, scratch.as_mut_slice(), &wrapper).map(|_| total - out.len())
        }
        .unwrap();
        assert_eq!(&output[..used], wrapper_bytes);
        let mut input = wrapper_bytes.as_slice();
        let decoded = TlvSeqWrapperFmt::decode(&mut input, scratch.as_mut_slice()).unwrap();
        assert_eq!(decoded, wrapper);
        assert!(input.is_empty());
    }

    #[test]
    fn test_ber_tlv_list_invalid_key_or_value_rejected_on_encode() {
        type TlvListFmt = BerTlvList<Vec<(String, String)>>;

        let invalid_key = vec![("bad".to_owned(), "1234".to_owned())];
        let invalid_value = vec![("t9F02_unknown".to_owned(), "12fg".to_owned())];
        let mut output = [0u8; 64];
        let mut scratch = [0u8; 64];
        let mut out = output.as_mut_slice();
        assert_eq!(
            error_kind(TlvListFmt::encode(&mut out, scratch.as_mut_slice(), &invalid_key)),
            Err(Error::Invalid)
        );
        let mut out = output.as_mut_slice();
        assert_eq!(
            error_kind(TlvListFmt::encode(&mut out, scratch.as_mut_slice(), &invalid_value)),
            Err(Error::Invalid)
        );
    }

    #[test]
    fn test_ber_tlv_decode_without_default() {
        let values = [
            TlvNoDefault {
                t59_code: "ABCD".into(),
                tdf23_tail: Some(FixedTail {
                    a: "123456".into(),
                    b: "12".into(),
                    c: "ABCD".into(),
                }),
            },
            TlvNoDefault {
                t59_code: "ABCD".into(),
                tdf23_tail: None,
            },
        ];
        let mut output = [0u8; 96];
        let mut scratch = [0u8; 96];

        for value in values {
            let used = {
                let total = output.len();
                let mut out = output.as_mut_slice();
                TlvNoDefaultFmt::encode(&mut out, scratch.as_mut_slice(), &value).map(|_| total - out.len())
            }
            .unwrap();
            let mut input = &output[..used];
            assert_eq!(TlvNoDefaultFmt::decode(&mut input, scratch.as_mut_slice()).unwrap(), value);
            assert!(input.is_empty());
        }
    }

    #[test]
    fn test_ber_tlv_with_extras_decode_without_default() {
        let value = TlvWithExtrasNoDefault {
            t59_code: "ABCD".into(),
            extras: BTreeMap::from([("t9F02_unknown".to_owned(), "1234".to_owned())]),
        };
        let mut output = [0u8; 96];
        let mut scratch = [0u8; 96];
        let used = {
            let total = output.len();
            let mut out = output.as_mut_slice();
            TlvWithExtrasNoDefaultFmt::encode(&mut out, scratch.as_mut_slice(), &value).map(|_| total - out.len())
        }
        .unwrap();
        let mut input = &output[..used];
        assert_eq!(
            TlvWithExtrasNoDefaultFmt::decode(&mut input, scratch.as_mut_slice()).unwrap(),
            value
        );
        assert!(input.is_empty());
    }

    #[test]
    fn test_union_format_decode_and_encode() {
        let mut scratch = [0u8; 16];

        let mut input = b"1234".as_slice();
        assert_eq!(
            UnionValueFmt::decode(&mut input, scratch.as_mut_slice()),
            Ok(UnionValue::Known("1234".into()))
        );
        assert!(input.is_empty());

        let mut input = b"ABCD".as_slice();
        assert_eq!(
            UnionValueFmt::decode(&mut input, scratch.as_mut_slice()),
            Ok(UnionValue::Alpha("ABCD".into()))
        );
        assert!(input.is_empty());

        let mut input = b"12AB".as_slice();
        let mut exact_scratch = [0u8; 4];
        assert_eq!(
            UnionValueFmt::decode(&mut input, exact_scratch.as_mut_slice()),
            Ok(UnionValue::Unknown("12AB".into()))
        );
        assert!(input.is_empty());

        let mut input = b"12".as_slice();
        assert_eq!(
            UnionValueFmt::decode(&mut input, scratch.as_mut_slice()),
            Ok(UnionValue::Unknown("12".into()))
        );
        assert!(input.is_empty());

        let mut input = b"12345".as_slice();
        assert_eq!(
            RestUnionValueFmt::decode(&mut input, scratch.as_mut_slice()),
            Ok(UnionValue::Unknown("12345".into()))
        );
        assert!(input.is_empty());

        let mut input = b"12".as_slice();
        let error = ShortUnionValueFmt::decode(&mut input, scratch.as_mut_slice()).unwrap_err();
        assert_eq!(error.kind, Error::UnexpectedEof);
        assert!(error.path().is_empty());
        assert_eq!(input, b"12");

        let mut output = [0u8; 16];
        let used = {
            let total = output.len();
            let mut out = output.as_mut_slice();
            UnionValueFmt::encode(&mut out, scratch.as_mut_slice(), &UnionValue::Known("1234".into())).unwrap();
            total - out.len()
        };
        assert_eq!(&output[..used], b"1234");

        let used = {
            let total = output.len();
            let mut out = output.as_mut_slice();
            UnionValueFmt::encode(&mut out, scratch.as_mut_slice(), &UnionValue::Unknown("ABCD".into())).unwrap();
            total - out.len()
        };
        assert_eq!(&output[..used], b"ABCD");

        let mut input = b"1234".as_slice();
        assert_eq!(
            BorrowedUnionValueFmt::decode(&mut input, scratch.as_mut_slice()),
            Ok(BorrowedUnionValue::Known("1234"))
        );
        assert!(input.is_empty());

        let mut input = b"\xC1\xC2\xC3\xC4".as_slice();
        assert_eq!(
            BorrowedUnionValueFmt::decode(&mut input, scratch.as_mut_slice()),
            Ok(BorrowedUnionValue::Unknown("ABCD"))
        );
        assert!(input.is_empty());

        let used = {
            let total = output.len();
            let mut out = output.as_mut_slice();
            BorrowedUnionValueFmt::encode(&mut out, scratch.as_mut_slice(), &BorrowedUnionValue::Unknown("ABCD")).unwrap();
            total - out.len()
        };
        assert_eq!(&output[..used], b"\xC1\xC2\xC3\xC4");
    }
}
mod enum_macros;
mod wrappers;

pub use scalar_serde::SerdeScalar;
#[doc(hidden)]
pub use scalar_serde::{decode_serde_scalar, encode_serde_scalar};

pub trait ListCountPolicy {
    fn encode_count(output: &mut &mut [u8], scratch: &mut &mut [u8], len: usize) -> Result<(), Error>;
    fn decode_count<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<Option<usize>, Error>;
}

pub trait ListSeparatorPolicy {
    const BYTE: Option<u8>;
}

pub trait BerTlvExtras {
    fn encode_unknowns(&self, output: &mut &mut [u8], scratch: &mut &mut [u8]) -> Result<(), Error>;
    fn decode_unknown(&mut self, tag: &[u8], value: &[u8], scratch: &mut &mut [u8]) -> Result<(), Error>;
}

#[inline(always)]
#[doc(hidden)]
pub fn advance_input(input: &mut &[u8], consumed: usize) -> Result<(), Error> {
    *input = input.split_off(consumed..).ok_or_else(|| {
        crate::utils::cold_path();
        Error::Internal
    })?;
    Ok(())
}

#[inline]
pub fn encode_delimiter(output: &mut &mut [u8], byte: u8) -> Result<(), Error> {
    let out = output.split_off_mut(..1).ok_or_else(|| {
        crate::utils::cold_path();
        Error::BufferOverflow
    })?;
    out[0] = byte;
    Ok(())
}

#[inline(always)]
pub fn wrap_struct_error<E: Into<StructError>>(error: E, field: &'static str) -> StructError {
    error.into().with_field(field)
}

#[inline(always)]
#[doc(hidden)]
pub fn encode_nested_value<T, F: CompositeFmt<T>>(value: &T, output: &mut &mut [u8], scratch: &mut &mut [u8]) -> Result<(), StructError> {
    F::encode_cursor(output, scratch, value)
}

#[inline(always)]
#[doc(hidden)]
pub fn encode_ber_tlv_field<F>(
    output: &mut &mut [u8],
    scratch: &mut &mut [u8],
    tag_hex: &str,
    field: &'static str,
    encode_value: F,
) -> Result<(), StructError>
where
    F: FnOnce(&mut &mut [u8], &mut &mut [u8]) -> Result<(), StructError>,
{
    let (tag_bytes, tag_len) = crate::primitive::bertlv::parse_hex_tag(tag_hex).map_err(|error| wrap_struct_error(error, field))?;
    let out = core::mem::take(output);
    let total = out.len();
    let mut value_out = &mut *out;
    encode_value(&mut value_out, scratch).map_err(|error| wrap_struct_error(error, field))?;
    let used = total - value_out.len();
    let head_len = tag_len + crate::primitive::bertlv::encoded_berlen(used).map_err(|error| wrap_struct_error(error, field))?;
    if total < head_len + used {
        crate::utils::cold_path();
        return Err(wrap_struct_error(Error::BufferOverflow, field));
    }
    out.copy_within(0..used, head_len);
    let mut head = &mut out[..head_len];
    crate::primitive::bertlv::encode_bertag(&mut head, &tag_bytes[..tag_len]).map_err(|error| wrap_struct_error(error, field))?;
    crate::primitive::bertlv::encode_berlen(&mut head, used).map_err(|error| wrap_struct_error(error, field))?;
    *output = &mut out[head_len + used..];
    Ok(())
}

#[inline(always)]
#[doc(hidden)]
pub fn decode_ber_tlv_field<'a, T, D>(
    tag_bytes: &[u8],
    tag_hex: &str,
    value_input: &mut &'a [u8],
    scratch: &mut &'a mut [u8],
    field_value: &mut Option<T>,
    field: &'static str,
    decode_value: D,
) -> Result<bool, StructError>
where
    D: FnOnce(&mut &'a [u8], &mut &'a mut [u8]) -> Result<T, StructError>,
{
    if !crate::primitive::bertlv::tag_eq_hex(tag_bytes, tag_hex).map_err(|error| wrap_struct_error(error, field))? {
        return Ok(false);
    }
    if field_value.is_some() {
        crate::utils::cold_path();
        return Err(wrap_struct_error(Error::Invalid, field));
    }
    let value = decode_value(value_input, scratch).map_err(|error| wrap_struct_error(error, field))?;
    if !value_input.is_empty() {
        crate::utils::cold_path();
        return Err(wrap_struct_error(Error::Invalid, field));
    }
    *field_value = Some(value);
    Ok(true)
}

#[inline(always)]
#[doc(hidden)]
pub fn should_retry_union(error: Error) -> bool {
    matches!(error, Error::Invalid | Error::InvalidValueLength | Error::UnexpectedEof)
}

#[inline(always)]
#[doc(hidden)]
pub fn decode_owned_struct<'de, T, F>(input: &mut &'de [u8], scratch: &'de mut [u8]) -> Result<T, StructError>
where
    F: CompositeFmt<T, Decoded<'de> = T>,
{
    F::decode(input, scratch)
}

#[inline]
fn encode_delimited_segment<E>(output: &mut &mut [u8], scratch: &mut &mut [u8], separator: u8, encode: E) -> Result<(), StructError>
where
    E: FnOnce(&mut &mut [u8], &mut &mut [u8]) -> Result<(), StructError>,
{
    let scratch_len = scratch.len();
    let used = {
        let mut segment_out = &mut **scratch;
        let mut nested_scratch = &mut output[..];
        encode(&mut segment_out, &mut nested_scratch)?;
        scratch_len - segment_out.len()
    };
    let segment = take_scratch(scratch, used)?;
    if contains_byte(segment, separator) {
        crate::utils::cold_path();
        return Err(Error::Invalid.into());
    }
    copy_bytes(output, segment)?;
    Ok(())
}

#[inline]
#[doc(hidden)]
pub fn encode_delimited_value<T, S: CompositeFmt<T>>(
    output: &mut &mut [u8],
    scratch: &mut &mut [u8],
    value: &T,
    separator: u8,
) -> Result<(), StructError> {
    encode_delimited_segment(output, scratch, separator, |segment_out, nested_scratch| {
        S::encode_cursor(segment_out, nested_scratch, value)
    })
}

#[inline]
#[doc(hidden)]
pub fn encode_delimited_context<T, C: ?Sized, S: ContextFmt<T, C>>(
    output: &mut &mut [u8],
    scratch: &mut &mut [u8],
    context: &C,
    value: &T,
    separator: u8,
) -> Result<(), StructError> {
    encode_delimited_segment(output, scratch, separator, |segment_out, nested_scratch| {
        S::encode_with(segment_out, nested_scratch, context, value)
    })
}

#[inline]
#[doc(hidden)]
pub fn encode_delimited_serde_value<T, F: ScalarFmt>(
    output: &mut &mut [u8],
    scratch: &mut &mut [u8],
    value: &T,
    separator: u8,
) -> Result<(), StructError>
where
    T: ?Sized + serde::Serialize,
{
    encode_delimited_segment(output, scratch, separator, |segment_out, nested_scratch| {
        encode_serde_scalar::<T, F>(value, segment_out, nested_scratch).map_err(StructError::from)
    })
}

#[inline]
fn decode_delimited_segment<'a, T, D>(segment: &'a [u8], scratch: &mut &'a mut [u8], decode: D) -> Result<T, StructError>
where
    D: FnOnce(&mut &'a [u8], &mut &'a mut [u8]) -> Result<T, StructError>,
{
    let mut input = segment;
    let value = decode(&mut input, scratch)?;
    if !input.is_empty() {
        crate::utils::cold_path();
        return Err(Error::Invalid.into());
    }
    Ok(value)
}

#[inline]
#[doc(hidden)]
pub fn decode_delimited_value<'a, T, S: CompositeFmt<T>>(
    segment: &'a [u8],
    scratch: &mut &'a mut [u8],
) -> Result<S::Decoded<'a>, StructError> {
    decode_delimited_segment(segment, scratch, |input, scratch| S::decode_cursor(input, scratch))
}

#[inline]
#[doc(hidden)]
pub fn decode_delimited_context<'a, T, C: ?Sized, S: ContextFmt<T, C>>(
    segment: &'a [u8],
    scratch: &mut &'a mut [u8],
    context: &C,
) -> Result<S::Decoded<'a>, StructError> {
    decode_delimited_segment(segment, scratch, |input, scratch| S::decode_with(input, scratch, context))
}

#[inline]
#[doc(hidden)]
pub fn decode_delimited_serde_value<'a, T, F: ScalarFmt>(segment: &'a [u8], scratch: &mut &'a mut [u8]) -> Result<T, StructError>
where
    T: serde::Deserialize<'a>,
{
    decode_delimited_segment(segment, scratch, |input, scratch| {
        decode_serde_scalar::<T, F>(input, scratch).map_err(StructError::from)
    })
}

#[inline]
#[doc(hidden)]
pub fn encode_delimited_literal<F: ScalarFmt>(
    output: &mut &mut [u8],
    scratch: &mut &mut [u8],
    expected: &[u8],
    separator: u8,
) -> Result<(), StructError> {
    encode_delimited_segment(output, scratch, separator, |segment_out, scratch| {
        F::encode(segment_out, scratch, expected).map_err(StructError::from)
    })
}

#[inline]
#[doc(hidden)]
pub fn decode_delimited_literal<'a, F: ScalarFmt>(
    segment: &'a [u8],
    scratch: &mut &'a mut [u8],
    expected: &[u8],
) -> Result<(), StructError> {
    decode_delimited_segment(segment, scratch, |input, scratch| {
        decode_literal::<F>(input, scratch, expected).map_err(StructError::from)
    })
}

#[inline(never)]
pub(crate) fn decode_variant<'a, T, E, F, W>(input: &mut &'a [u8], scratch: &mut &'a mut [u8], wrap: W) -> Result<E, StructError>
where
    F: CompositeFmt<T>,
    W: FnOnce(F::Decoded<'a>) -> E,
{
    Ok(wrap(F::decode_cursor(input, scratch)?))
}

#[inline]
pub fn decode_literal<'a, F: ScalarFmt>(input: &mut &'a [u8], scratch: &mut &'a mut [u8], expected: &[u8]) -> Result<(), Error> {
    let source = *input;
    let mut input_ptr = source;
    let decoded = F::decode(&mut input_ptr, scratch)?;
    advance_input(input, source.len() - input_ptr.len())?;
    if decoded != expected {
        crate::utils::cold_path();
        return Err(Error::Invalid);
    }
    Ok(())
}

#[inline]
pub fn match_literal<'a, F: ScalarFmt>(input: &mut &'a [u8], scratch: &mut &'a mut [u8], expected: &[u8]) -> Result<bool, Error> {
    let encoded_len = F::encoded_len(expected)?;

    let mut stack = [0u8; 64];
    if encoded_len <= stack.len() {
        let encoded = stack.get_mut(..encoded_len).ok_or_else(|| {
            crate::utils::cold_path();
            Error::Internal
        })?;
        let mut encoded_out = &mut encoded[..];
        let mut scratch_ptr = &mut **scratch;
        F::encode(&mut encoded_out, &mut scratch_ptr, expected)?;
        if !encoded_out.is_empty() {
            crate::utils::cold_path();
            return Err(Error::Internal);
        }
        return match_encoded_literal(input, encoded);
    }

    let mut scratch_ptr = &mut **scratch;
    let encoded = take_scratch(&mut scratch_ptr, encoded_len)?;
    let mut encoded_out = &mut encoded[..];
    F::encode(&mut encoded_out, &mut scratch_ptr, expected)?;
    if !encoded_out.is_empty() {
        crate::utils::cold_path();
        return Err(Error::Internal);
    }
    match_encoded_literal(input, encoded)
}

#[inline]
fn match_encoded_literal(input: &mut &[u8], encoded: &[u8]) -> Result<bool, Error> {
    if input.len() < encoded.len() {
        if encoded.starts_with(input) {
            crate::utils::cold_path();
            return Err(Error::UnexpectedEof);
        }
        return Ok(false);
    }

    let head = input.get(..encoded.len()).ok_or_else(|| {
        crate::utils::cold_path();
        Error::Internal
    })?;
    if head != encoded {
        return Ok(false);
    }
    advance_input(input, encoded.len())?;
    Ok(true)
}
