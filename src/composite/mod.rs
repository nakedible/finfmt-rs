//! Composite formats and the field dispatch traits.
//!
//! A format for values of type `T` implements [`FieldEncode<T>`] and
//! [`FieldDecode<'de, T>`]. Every [`ScalarFmt`] is such a format for the value
//! types implementing [`ScalarEncode`]/[`ScalarDecode`]: strings, `&str` and
//! integers, and `CompactString` with the `compact_str` feature. Serde values
//! go through [`SerdeScalar<F>`], spelled out per field. The composites here
//! take their inner formats through the same traits, so scalar and composite
//! formats nest alike.
//!
//! Record macros take entries of these forms:
//! - `field: Fmt`, a field encoded by `Fmt`;
//! - `field(context): Fmt`, a field whose format also reads an earlier field;
//! - `field: Option<Fmt>`, a field the wire container can omit;
//! - `_: Fmt = b"…"`, a literal encoded by `Fmt`.
//!
//! `Option<Fmt>` is container-level presence, not merely a field whose value
//! happens to be `Option<T>`. In `concat_format!`, optional fields are
//! tail-only and decode to `None` after EOF. In `delimited_format!`, an empty
//! segment decodes to `None`. In bitmap and BER-TLV formats, absence is
//! controlled by the bitmap bit or tag presence. If bytes are always present but
//! a pattern inside those bytes means "no value", express that in the field
//! format, for example with `OptionAs`, not by wrapping the macro field in
//! `Option<...>`.

use core::marker::PhantomData;

use crate::primitive::bytes::{copy_bytes, take_delimited};
use crate::utils::split_scratch;
use crate::{CompositeError, Error, ScalarFmt};

/// Encode a value of type `T`.
///
/// Encoding advances `output` past what it writes, so formats compose: a nested
/// format writes its part of the parent's output. Scratch is a workspace for
/// this call only. Use [`crate::encode`] for a whole message.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a format for values of type `{T}`",
    label = "no `FieldEncode<{T}>` implementation",
    note = "scalar formats encode strings and integers; use `SerdeScalar<Fmt>` for a serde value, or implement `ScalarEncode` for your own type"
)]
pub trait FieldEncode<T: ?Sized> {
    fn encode_field(output: &mut &mut [u8], scratch: &mut [u8], value: &T) -> Result<(), CompositeError>;
}

/// Decode a value of type `T`, which may borrow from input or scratch for `'de`.
///
/// Decoding advances `input` past what it consumes. Scratch is an arena that
/// decoded values may borrow from: the callee advances it past every byte it
/// keeps. Use [`crate::decode`] for a whole message.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a format for values of type `{T}`",
    label = "no `FieldDecode<{T}>` implementation",
    note = "scalar formats decode strings and integers; use `SerdeScalar<Fmt>` for a serde value, or implement `ScalarDecode` for your own type"
)]
pub trait FieldDecode<'de, T> {
    /// The format consumes all remaining input, as a [`crate::Rest`] length
    /// does. An absent pattern for such a field must match the whole
    /// remainder.
    const TAKES_REST: bool = false;

    fn decode_field(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<T, CompositeError>;
}

/// Encode a whole message, returning the number of bytes written to `output`.
#[inline]
pub fn encode<F: FieldEncode<T>, T: ?Sized>(output: &mut [u8], scratch: &mut [u8], value: &T) -> Result<usize, CompositeError> {
    let capacity = output.len();
    let mut cursor = output;
    F::encode_field(&mut cursor, scratch, value)?;
    Ok(capacity - cursor.len())
}

/// Decode a whole message. Input left over after the value is `Invalid`.
#[inline]
pub fn decode<'de, F: FieldDecode<'de, T>, T>(input: &'de [u8], scratch: &'de mut [u8]) -> Result<T, CompositeError> {
    let mut input = input;
    let mut scratch = scratch;
    let value = F::decode_field(&mut input, &mut scratch)?;
    if !input.is_empty() {
        crate::utils::cold_path();
        return Err(Error::Invalid.into());
    }
    Ok(value)
}

/// Encode into scratch, for a format that must know an encoding's bytes before
/// writing what precedes them, such as a length.
///
/// `encode` writes into the first half of scratch, with the second half as its
/// workspace. Returns the bytes it wrote and the scratch after them. Workspace
/// never shares a region with bytes bound for output, and output is only
/// appended to, so a length error cannot ship leftover workspace. Nested
/// staging halves scratch again, so scratch must hold about `2^depth` times the
/// largest staged encoding.
#[inline(always)]
pub fn encode_staged<E, F>(scratch: &mut [u8], encode: F) -> Result<(&mut [u8], &mut [u8]), E>
where
    E: From<Error>,
    F: FnOnce(&mut &mut [u8], &mut [u8]) -> Result<(), E>,
{
    let used = {
        let half = scratch.len() / 2;
        let (area, workspace) = split_scratch(scratch, half)?;
        let mut staged = area;
        let available = staged.len();
        encode(&mut staged, workspace)?;
        available - staged.len()
    };
    Ok(split_scratch(scratch, used)?)
}

/// Encode a value using an already available context value, such as an
/// earlier field that selects the layout.
pub trait ContextEncode<T: ?Sized, C: ?Sized> {
    fn encode_with(output: &mut &mut [u8], scratch: &mut [u8], context: &C, value: &T) -> Result<(), CompositeError>;
}

/// Decode a value using an already available context value.
pub trait ContextDecode<'de, T, C: ?Sized> {
    fn decode_with(input: &mut &'de [u8], scratch: &mut &'de mut [u8], context: &C) -> Result<T, CompositeError>;
}

/// The wire encoding of a value that is absent although its bytes are on the
/// wire: a blank-filled amount, a zero date, COBOL low-values.
///
/// Absent encodings are wire bytes, matched before the value's format sees the
/// input: that format often cannot represent them. A present value that
/// encodes to the same bytes reads back as absent, which is the format's
/// definition when the pattern is a valid value, such as a zero amount.
pub trait AbsentFmt {
    /// Write the absent encoding, advancing `output`.
    fn encode_absent(output: &mut &mut [u8], scratch: &mut [u8]) -> Result<(), Error>;

    /// Whether `input` starts with an absent encoding. If so, advance `input`
    /// past it, which may be no bytes at all. `scratch` is a workspace for this
    /// call only.
    ///
    /// The default stages the absent encoding with [`encode_staged`] and
    /// compares it with the input. Override it to accept other spellings.
    #[inline]
    fn decode_absent(input: &mut &[u8], scratch: &mut [u8]) -> Result<bool, Error> {
        let (absent, _) = encode_staged(scratch, Self::encode_absent)?;
        Ok(match_prefix(input, absent))
    }
}

/// Advance `input` past `prefix` if it starts with it.
#[inline(always)]
fn match_prefix(input: &mut &[u8], prefix: &[u8]) -> bool {
    match input.strip_prefix(prefix) {
        Some(rest) => {
            *input = rest;
            true
        }
        None => false,
    }
}

/// An absent value encoded as the constant bytes of `P`, for example
/// `AbsentBytes<Fill<0x40, 12>>` for twelve EBCDIC spaces.
pub struct AbsentBytes<P>(PhantomData<P>);

impl<P: crate::ConstBytes> AbsentFmt for AbsentBytes<P> {
    #[inline(always)]
    fn encode_absent(output: &mut &mut [u8], _scratch: &mut [u8]) -> Result<(), Error> {
        copy_bytes(output, P::BYTES)?;
        Ok(())
    }

    #[inline(always)]
    fn decode_absent(input: &mut &[u8], _scratch: &mut [u8]) -> Result<bool, Error> {
        Ok(match_prefix(input, P::BYTES))
    }
}

/// Encode/decode an inner format's value through an outer scalar field. A group of
/// fields or a list with a byte length is a frame whose field has that length,
/// such as `Frame<Field<Binary<0, 999>, AsciiLength<3>>, Inner>`.
pub struct Frame<F, S>(PhantomData<(F, S)>);
/// A fixed physical body whose length prefix excludes absent trailing fields.
///
/// `Len` states the used body extent in bytes. The full body, including absent filler, remains on the
/// wire. The declared extent must end at a field boundary, and excluded fields
/// must match their absent encoding.
pub struct TrailingLengthFrame<Len, Body, Tails, const BASE_LEN: usize>(PhantomData<(Len, Body, Tails)>);
/// A trailing field `WIDTH` bytes wide whose absence the `Absent` encoding
/// shows, followed by the `Rest` of the trailing fields.
pub struct TrailingField<Absent, const WIDTH: usize, Rest = NoTrailingFields>(PhantomData<(Absent, Rest)>);
pub struct NoTrailingFields;
/// Encode nothing, and decode the value's `Default`.
pub struct Empty;
/// Structural BER-TLV representation as a Serde map or sequence of
/// `(tag, value)` pairs. Tags and values are uppercase hex, such as `"9F02"` and
/// `"000000012345"`; an empty value is `""`. Only `ber_tlv_format!` extras, which
/// sit beside named fields, use `t9F02_unknown` keys.
///
/// Order and duplicates are preserved if the chosen collection type preserves
/// them: a sequence keeps every entry, while a std map keeps the last value of a
/// repeated tag. To catch repeats, decode into a sequence or into a map type
/// that rejects duplicate keys, such as `serde_with`'s `MapPreventDuplicates`.
/// Decoding is strict by default. Set `ALLOW_ZERO_PADDING` to accept `00`
/// bytes before, between and after entries. Values are never trimmed, and
/// encoding never emits padding.
#[cfg(feature = "serde")]
pub struct BerTlvList<const ALLOW_ZERO_PADDING: bool = false>;
/// A list of `MIN` to `MAX` items, whose length `L` counts items: a prefix
/// such as [`crate::AsciiLength`], or [`crate::Fixed`], states the item count,
/// and [`crate::Rest`] takes the rest of the input. A list with a byte length
/// is a [`Frame`] around a list that takes the rest.
///
/// `Sep` is `()` or a [`Separator`] between items. Encoding a list outside
/// `MIN..=MAX` items is `InvalidValueLength`.
///
/// Without separators, items must provide their own boundaries, and items of a
/// list without a count must consume at least one byte. With separators, a
/// counted list's last item consumes the remaining input, so that input must be
/// bounded externally, and only that item may contain the separator. An empty
/// byte extent is an empty list, so a single item that encodes to no bytes is
/// ambiguous. Item checks must keep separators out of values; debug builds
/// assert it.
pub struct BoundedList<L, Item, Sep, const MIN: usize, const MAX: usize>(PhantomData<(L, Item, Sep)>);
/// An `Option` whose value is always on the wire: `Some` is encoded by
/// `Inner`, and `None` by the `Absent` encoding, which decoding matches first.
/// If it does not match, `Inner` decodes and its errors are returned, never
/// read as absent. For an `Inner` that takes the rest of the input, the absent
/// encoding must match all of it.
pub struct OptionAs<Inner, Absent>(PhantomData<(Inner, Absent)>);
/// A fixed physical area of `MAX` slots of `WIDTH` bytes each, with a
/// separately declared used extent.
///
/// `Len` states the number of used slots. [`crate::Rest`] cannot recover it and
/// fails to build. Slots inside that extent hold values encoded by `Inner`;
/// each remaining slot holds exactly one `Absent` encoding. `WIDTH` must be
/// nonzero:
///
/// ```compile_fail
/// # use finfmt::composite::{AbsentBytes, Empty, FieldDecode, FixedAreaList};
/// # use finfmt::{Fill, Fixed};
/// type ZeroWidth = FixedAreaList<Fixed<0>, Empty, AbsentBytes<Fill<b' ', 0>>, 0, 1>;
/// let _ = <ZeroWidth as FieldDecode<'_, Vec<()>>>::decode_field(&mut &b""[..], &mut &mut [][..]);
/// ```
pub struct FixedAreaList<Len, Inner, Absent, const WIDTH: usize, const MAX: usize>(PhantomData<(Len, Inner, Absent)>);
pub struct Separator<const BYTE: u8>;

mod bertlv;
#[doc(hidden)]
pub use bertlv::decode_ber_tlv_collection_entry;
mod bertlv_macros;
#[cfg(feature = "serde")]
mod bertlv_serde;
mod bitmap_macros;
mod concat_macros;
mod delimited_macros;
mod repeated;
mod scalar;
#[cfg(feature = "serde")]
mod scalar_serde;
pub use scalar::{ScalarDecode, ScalarEncode};
#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde::{Deserialize, Serialize};

    use super::*;
    use crate::field::Length;
    use crate::primitive::nibble::{BcdzDigits, UpperHexDigits};
    use crate::{
        Ascii, AsciiLength, Binary, Count, Ebcdic037, EbcdicLength, Error, Field, Fixed, Numeric, PadLeft, PadRightEven, SignPrefix,
        Track2, UnpackNibbles,
    };

    type N6 = Field<Numeric<6, 6>, Fixed<6>>;
    type N2 = Field<Numeric<2, 2>, Fixed<2>>;
    type A2 = Field<Ascii<2, 2>, Fixed<2>>;
    type A3 = Field<Ascii<3, 3>, Fixed<3>>;
    type A4 = Field<crate::Ascii<4, 4>, Fixed<4>>;
    type BitmapBinaryWord = crate::Identity;
    type Track2Fmt = Field<Track2<1, 37>, EbcdicLength<2>, crate::chain!(PadRightEven<b'?'>, crate::PackNibblesLeft<BcdzDigits, 0x0F>)>;
    const PIPE_SEPARATOR: u8 = b'|';

    fn error_kind<T>(result: Result<T, CompositeError>) -> Result<T, Error> {
        result.map_err(|error| error.kind)
    }
    type AmountFmt =
        SignPrefix<Field<Numeric<1, 16>, Fixed<16>, crate::chain!(PadLeft<16, b'0', 1>, Count, crate::PackNibblesRight<BcdzDigits, 0>)>>;

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
            inner: Option<FixedTailFmt>,
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
        struct BorrowedBitmapFmt for<'a> BorrowedBitmap<'a>, crate::bitmap::BitmapLayout::bits(32), BitmapBinaryWord {
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
            "DF23" => tail: Option<BorrowedConcatFmt>,
        }
    }

    #[test]
    fn test_direct_borrowed_record_syntax() {
        #[derive(Debug, PartialEq)]
        struct Record<'a> {
            ascii: &'a str,
            ebcdic: Option<&'a str>,
        }
        crate::concat_format! { struct Concat for<'a> Record<'a> {
            ascii: A4, ebcdic: Option<A4Ebcdic>,
        } }
        crate::delimited_format! { struct Delimited for<'a> Record<'a>, b'|' {
            ascii: A4, ebcdic: Option<A4Ebcdic >,
        } }
        crate::bitmap_format! { struct Bitmap for<'a> Record<'a>, crate::bitmap::BitmapLayout::iso(1, 1), BitmapBinaryWord {
            head: { ascii: A4, }
            3 => ebcdic: Option<A4Ebcdic>,
        } }
        const ASCII_TAG: &str = "59";
        crate::ber_tlv_format! { struct Ber for<'a> Record<'a> {
            ASCII_TAG => ascii: A4, "5A" => ebcdic: Option<A4Ebcdic >,
        } }
        fn check<F: for<'a> FieldEncode<Record<'a>> + for<'de> FieldDecode<'de, Record<'de>>>() {
            for ebcdic in [None, Some("WXYZ")] {
                let value = Record { ascii: "ABCD", ebcdic };
                let mut output = [0; 32];
                let mut scratch = [0; 32];
                let used = {
                    let mut out = output.as_mut_slice();
                    F::encode_field(&mut out, &mut scratch, &value).unwrap();
                    32 - out.len()
                };
                let mut input = &output[..used];
                let decoded: Record<'_> = F::decode_field(&mut input, &mut &mut scratch[..]).unwrap();
                assert_eq!(decoded, value);
                assert!(input.is_empty());
                assert!((output.as_ptr_range()).contains(&decoded.ascii.as_ptr()));
                if let Some(text) = decoded.ebcdic {
                    let ptr = text.as_ptr();
                    assert!((scratch.as_ptr_range()).contains(&ptr));
                }
            }
        }
        check::<Concat>();
        check::<Delimited>();
        check::<Bitmap>();
        check::<Ber>();
    }

    type FramedFixedTailFmt = Frame<Field<Ascii<0, 12>, AsciiLength<2>>, FixedTailFmt>;
    type FramedHexFixedTailFmt = Frame<Field<Binary<0, 12>, AsciiLength<2>, UnpackNibbles<UpperHexDigits>>, FixedTailFmt>;
    type Blank3 = AbsentBytes<crate::Fill<b' ', 3>>;
    type OptionalA3SpaceFmt = OptionAs<A3, Blank3>;

    #[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    struct FramedConcat {
        head: String,
        inner: FixedTail,
        tail: String,
    }

    crate::concat_format! {
        struct FramedConcatFmt for FramedConcat {
            head: N2,
            inner: FramedFixedTailFmt,
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
            inner: FramedHexFixedTailFmt,
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
            tail1: OptionalA3SpaceFmt,
            tail2: OptionalA3SpaceFmt,
        }
    }

    type TrailingLengthTails = TrailingField<Blank3, 3, TrailingField<Blank3, 3>>;
    type TrailingLengthDataFmt = TrailingLengthFrame<AsciiLength<2>, TrailingLengthBodyFmt, TrailingLengthTails, 2>;

    #[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    struct TlvData {
        t59_code: String,
        tdf23_tail: Option<FixedTail>,
    }

    crate::ber_tlv_format! {
        #[doc = "Test format for a named BER-TLV struct."]
        struct TlvDataFmt for TlvData {
            "59" => t59_code: A4,
            "DF23" => tdf23_tail: Option<FixedTailFmt>,
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
            "DF23" => tdf23_tail: Option<FixedTailFmt>,
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
            Known(FixedNumeric4),
            Alpha(FixedAlpha4),
            Unknown(RestAscii8),
        }
    }

    crate::union_format! {
        struct RestUnionValueFmt for UnionValue {
            Known(RestNumeric4),
            Alpha(FixedAlpha4),
            Unknown(RestAscii8),
        }
    }

    #[derive(Debug, PartialEq, Eq, Serialize)]
    enum BorrowedUnionValue<'a> {
        Known(&'a str),
        Unknown(&'a str),
    }

    crate::union_format! {
        struct BorrowedUnionValueFmt for<'a> BorrowedUnionValue<'a> {
            Known(FixedNumeric4),
            Unknown(A4Ebcdic),
        }
    }

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    enum ShortUnionValue {
        Numeric(String),
        Alpha(String),
    }

    crate::union_format! {
        struct ShortUnionValueFmt for ShortUnionValue {
            Numeric(FixedNumeric4),
            Alpha(FixedAlpha4),
        }
    }

    #[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    struct WithBitmapTlv {
        f003_processing_code: String,
        f048_details: Option<TlvData>,
    }

    crate::bitmap_format! {
        #[doc = "Test format for a nested BER-TLV field inside a bitmap."]
        struct WithBitmapTlvFmt for WithBitmapTlv, crate::bitmap::BitmapLayout::iso(1, 1), BitmapBinaryWord {
            3 => f003_processing_code: N6,
            48 => f048_details: Option<TlvDataFmt>,
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
        struct Auth1100Fmt for Auth1100, crate::bitmap::BitmapLayout::iso(1, 2), BitmapBinaryWord {
            3 => f003_processing_code: N6,
            11 => f011_stan: N6,
            35 => f035_track2_data: Option<Track2Fmt>,
            48 => f048_fixed_tail: Option<FixedTailFmt>,
            97 => f097_amount_net_settlement: Option<AmountFmt>,
        }
    }

    #[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    struct LocalBitmapData {
        a: Option<String>,
        b: Option<String>,
    }

    crate::bitmap_format! {
        struct LocalBitmapDataFmt for LocalBitmapData, crate::bitmap::BitmapLayout::bits(32), BitmapBinaryWord {
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
        struct BitmapNoDefaultFmt for BitmapNoDefault, crate::bitmap::BitmapLayout::bits(32), BitmapBinaryWord {
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
        struct SerdeScalarBitmapDataFmt for SerdeScalarBitmapData, crate::bitmap::BitmapLayout::iso(1, 1), BitmapBinaryWord {
            3 => f003_processing_code: SerdeScalar<N6>,
            11 => f011_stan: Option<SerdeScalar<N6>>,
        }
    }

    #[derive(Debug, Default, PartialEq, Eq)]
    struct ManualProcessingCode(String);

    impl ScalarEncode for ManualProcessingCode {
        fn encode_scalar<F: ScalarFmt>(&self, output: &mut &mut [u8], scratch: &mut [u8]) -> Result<(), Error> {
            F::encode_str(output, scratch, &self.0)
        }
    }

    impl<'de> ScalarDecode<'de> for ManualProcessingCode {
        fn decode_scalar<F: ScalarFmt>(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self, Error> {
            Ok(Self(F::decode_str(input, scratch)?.to_owned()))
        }
    }

    #[derive(Debug, Default, PartialEq, Eq)]
    struct ManualStan(u32);

    impl ScalarEncode for ManualStan {
        fn encode_scalar<F: ScalarFmt>(&self, output: &mut &mut [u8], scratch: &mut [u8]) -> Result<(), Error> {
            F::encode_u64(output, scratch, u64::from(self.0))
        }
    }

    impl<'de> ScalarDecode<'de> for ManualStan {
        fn decode_scalar<F: ScalarFmt>(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self, Error> {
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
        struct ManualScalarBitmapDataFmt for ManualScalarBitmapData, crate::bitmap::BitmapLayout::iso(1, 1), BitmapBinaryWord {
            3 => f003_processing_code: N6,
            11 => f011_stan: Option<N6>,
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
            serde_value: SerdeScalar<N2>,
            direct_value: N2,
            nested_value: FixedTailFmt,
            optional_serde: Option<SerdeScalar<N2>>,
            optional_direct: Option<N2>,
            optional_nested: Option<FixedTailFmt>,
            optional_direct_inline: Option<Field<Numeric<2, 2>, Fixed<2>>>,
            optional_nested_inline: Option<Frame<Field<Ascii<0, 12>, AsciiLength<2>>, FixedTailFmt>>,
        }
    }

    crate::delimited_format! {
        struct FieldSyntaxDelimitedFmt for FieldSyntaxRecord, PIPE_SEPARATOR {
            serde_value: SerdeScalar<N2>,
            direct_value: N2,
            nested_value: FixedTailFmt,
            optional_serde: Option<SerdeScalar<N2>>,
            optional_direct: Option<N2>,
            optional_nested: Option<FixedTailFmt>,
            optional_direct_inline: Option<Field<Numeric<2, 2>, Fixed<2>>>,
            optional_nested_inline: Option<Frame<Field<Ascii<0, 12>, AsciiLength<2>>, FixedTailFmt>>,
        }
    }

    crate::bitmap_format! {
        struct FieldSyntaxBitmapFmt for FieldSyntaxRecord, crate::bitmap::BitmapLayout::iso(1, 1), BitmapBinaryWord {
            2 => serde_value: SerdeScalar<N2>,
            3 => direct_value: N2,
            4 => nested_value: FixedTailFmt,
            5 => optional_serde: Option<SerdeScalar<N2>>,
            6 => optional_direct: Option<N2>,
            7 => optional_nested: Option<FixedTailFmt>,
            8 => optional_direct_inline: Option<Field<Numeric<2, 2>, Fixed<2>>>,
            9 => optional_nested_inline: Option<Frame<Field<Ascii<0, 12>, AsciiLength<2>>, FixedTailFmt>>,
        }
    }

    crate::ber_tlv_format! {
        struct FieldSyntaxBerTlvFmt for FieldSyntaxRecord {
            "02" => serde_value: SerdeScalar<N2>,
            "03" => direct_value: N2,
            "04" => nested_value: FixedTailFmt,
            "05" => optional_serde: Option<SerdeScalar<N2>>,
            "06" => optional_direct: Option<N2>,
            "07" => optional_nested: Option<FixedTailFmt>,
            "08" => optional_direct_inline: Option<Field<Numeric<2, 2>, Fixed<2>>>,
            "09" => optional_nested_inline: Option<Frame<Field<Ascii<0, 12>, AsciiLength<2>>, FixedTailFmt>>,
        }
    }

    #[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(transparent)]
    struct DualStan(u8);

    impl ScalarEncode for DualStan {
        fn encode_scalar<F: ScalarFmt>(&self, output: &mut &mut [u8], scratch: &mut [u8]) -> Result<(), Error> {
            F::encode_u64(output, scratch, u64::from(self.0) + 10)
        }
    }

    impl<'de> ScalarDecode<'de> for DualStan {
        fn decode_scalar<F: ScalarFmt>(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self, Error> {
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
        struct SerdeDualStanDataFmt for SerdeDualStanData, crate::bitmap::BitmapLayout::iso(1, 1), BitmapBinaryWord {
            11 => f011_stan: SerdeScalar<N2>,
        }
    }

    crate::bitmap_format! {
        struct ManualDualStanDataFmt for ManualDualStanData, crate::bitmap::BitmapLayout::iso(1, 1), BitmapBinaryWord {
            11 => f011_stan: N2,
        }
    }

    #[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    struct BoolScalarData {
        value: bool,
    }

    crate::concat_format! {
        struct BoolScalarDataFmt for BoolScalarData {
            value: SerdeScalar<N2>,
        }
    }

    #[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    struct BytesScalarData {
        value: Vec<u8>,
    }

    crate::concat_format! {
        struct BytesScalarDataFmt for BytesScalarData {
            value: SerdeScalar<A4>,
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
            value: SerdeScalar<A4>,
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
            selector: VariantSelectorFmt,
            body(selector): RetainedVariantDataFmt,
            suffix: A4,
        }
    }

    crate::delimited_format! {
        struct DelimitedRetainedVariantRecordFmt for RetainedVariantRecord, b'|' {
            selector: VariantSelectorFmt,
            body(selector): RetainedVariantDataFmt,
            suffix: A4,
        }
    }

    #[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    struct DelimitedSlots {
        first: String,
        second: String,
        third: String,
    }

    type DelimitedSecond = Field<Ascii<0, 3>, crate::Fixed<3>, crate::chain!(crate::PadRight<3, b' '>)>;

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

    type A5Padded = Field<Ascii<0, 5>, crate::Fixed<5>, crate::chain!(crate::PadRight<5, b' '>)>;
    type CountN2 = Field<Numeric<1, 2>, Fixed<2>, PadLeft<2, b'0', 1>>;
    type CountedAsciiListFmt = Frame<Field<Binary<7, 19>, AsciiLength<2>>, BoundedList<AsciiLength<2>, A5Padded, Separator<b'/'>, 0, 3>>;
    type ScalarCountedAsciiListFmt =
        Frame<Field<Binary<7, 19>, AsciiLength<2>>, BoundedList<Length<CountN2>, A5Padded, Separator<b'/'>, 0, 3>>;
    type FixedAsciiListFmt = BoundedList<Fixed<3>, A5Padded, (), 3, 3>;

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(transparent)]
    struct NoDefaultAbsentValue(String);

    type Blank4 = AbsentBytes<crate::Fill<b' ', 4>>;
    type OptionalNoDefaultAbsentFmt = OptionAs<SerdeScalar<A4>, Blank4>;
    type RepeatedNoDefaultByteFillFmt = FixedAreaList<EbcdicLength<1>, SerdeScalar<A4>, Blank4, 4, 2>;

    /// An absent encoding given as a value through the field's format.
    struct NoDefaultLiteralAbsentFmt;
    impl AbsentFmt for NoDefaultLiteralAbsentFmt {
        fn encode_absent(output: &mut &mut [u8], scratch: &mut [u8]) -> Result<(), Error> {
            <A4 as FieldEncode<str>>::encode_field(output, scratch, "NONE").map_err(|error| error.kind)
        }
    }

    type OptionalNoDefaultLiteralAbsentFmt = OptionAs<SerdeScalar<A4>, NoDefaultLiteralAbsentFmt>;
    type RepeatedNoDefaultAbsentFmt = FixedAreaList<EbcdicLength<1>, SerdeScalar<A4>, NoDefaultLiteralAbsentFmt, 4, 2>;

    #[test]
    fn test_whole_message_entry_points() {
        let value = ConcatNoDefault {
            first: "12".into(),
            second: "ABCD".into(),
        };
        let mut output = [0xEE; 16];
        let used = crate::encode::<ConcatNoDefaultFmt, _>(&mut output, &mut [], &value).unwrap();
        assert_eq!(&output[..used], b"HEAD12ABCD");
        assert_eq!(output[used], 0xEE);
        assert_eq!(crate::decode::<ConcatNoDefaultFmt, _>(&output[..used], &mut []), Ok(value));
        assert_eq!(
            crate::decode::<ConcatNoDefaultFmt, _>(&output[..used + 1], &mut [])
                .unwrap_err()
                .kind,
            Error::Invalid
        );
        assert_eq!(
            crate::encode::<ConcatNoDefaultFmt, _>(
                &mut output[..9],
                &mut [],
                &ConcatNoDefault {
                    first: "12".into(),
                    second: "ABCD".into(),
                }
            )
            .unwrap_err()
            .kind,
            Error::BufferOverflow
        );
    }

    #[test]
    fn test_bounded_list_roundtrip_and_edges() {
        let value = vec!["ABC".to_owned(), "DEF".to_owned(), "GHI".to_owned()];
        let mut output = [0u8; 64];
        let mut scratch = [0u8; 64];
        let total = output.len();
        let used = {
            let mut out = output.as_mut_slice();
            CountedAsciiListFmt::encode_field(&mut out, scratch.as_mut_slice(), &value).unwrap();
            total - out.len()
        };
        assert_eq!(&output[..used], b"1903ABC  /DEF  /GHI  ");

        let mut input = &output[..used];
        let decoded: Vec<String> = CountedAsciiListFmt::decode_field(&mut input, &mut scratch.as_mut_slice()).unwrap();
        assert!(input.is_empty());
        assert_eq!(decoded, value);

        let used = {
            let mut out = output.as_mut_slice();
            ScalarCountedAsciiListFmt::encode_field(&mut out, scratch.as_mut_slice(), &value).unwrap();
            total - out.len()
        };
        assert_eq!(&output[..used], b"1903ABC  /DEF  /GHI  ");

        let mut input = &output[..used];
        let decoded: Vec<String> = ScalarCountedAsciiListFmt::decode_field(&mut input, &mut scratch.as_mut_slice()).unwrap();
        assert!(input.is_empty());
        assert_eq!(decoded, value);

        let used = {
            let mut out = output.as_mut_slice();
            FixedAsciiListFmt::encode_field(&mut out, scratch.as_mut_slice(), &value).unwrap();
            total - out.len()
        };
        assert_eq!(&output[..used], b"ABC  DEF  GHI  ");

        let mut input = &output[..used];
        let decoded: Vec<String> = FixedAsciiListFmt::decode_field(&mut input, &mut scratch.as_mut_slice()).unwrap();
        assert!(input.is_empty());
        assert_eq!(decoded, value);

        let too_many = vec!["A".to_owned(), "B".to_owned(), "C".to_owned(), "D".to_owned()];
        let mut out = output.as_mut_slice();
        assert_eq!(
            error_kind(CountedAsciiListFmt::encode_field(&mut out, scratch.as_mut_slice(), &too_many)),
            Err(Error::InvalidValueLength)
        );
        let mut out = output.as_mut_slice();
        assert_eq!(
            error_kind(FixedAsciiListFmt::encode_field(&mut out, scratch.as_mut_slice(), &too_many)),
            Err(Error::InvalidValueLength)
        );

        let mut invalid = b"1903ABC  XDEF  XGHI  ".as_slice();
        assert_eq!(
            error_kind::<Vec<String>>(CountedAsciiListFmt::decode_field(&mut invalid, &mut scratch.as_mut_slice())),
            Err(Error::Invalid)
        );
    }

    #[test]
    fn test_delimited_output_and_workspace() {
        #[derive(Debug)]
        struct Record {
            first: String,
            second: String,
        }
        crate::delimited_format! { struct Format for Record, b'|' { first: A2, second: A2, } }
        let value = Record {
            first: "AB".into(),
            second: "CD".into(),
        };
        for capacity in 0..=6 {
            let mut output = [0xFF; 6];
            let mut out = &mut output[..capacity];
            let result = error_kind(Format::encode_field(&mut out, &mut [], &value));
            if capacity < 5 {
                assert_eq!(result, Err(Error::BufferOverflow));
            } else {
                assert_eq!(result, Ok(()));
                assert_eq!(out.len(), capacity - 5);
                assert_eq!(&output[..5], b"AB|CD");
                assert_eq!(output[5], 0xFF);
            }
        }
        type Compressed =
            Field<Numeric<1, 4>, crate::Fixed<4>, crate::chain!(PadLeft<4, b'0', 1>, Count, crate::PackNibblesRight<BcdzDigits, 0>)>;
        for capacity in 0..=5 {
            let mut output = [0xFF; 3];
            let mut scratch = [0; 5];
            let mut out = output.as_mut_slice();
            let result = error_kind(encode_delimited_value::<_, Compressed>(
                &mut out,
                &mut scratch[..capacity],
                &"1".to_owned(),
                Some(b'|'),
            ));
            if capacity < 4 {
                assert_eq!(result, Err(Error::BufferOverflow));
            } else {
                assert_eq!(result, Ok(()));
                assert_eq!(out, &[0xFF]);
                assert_eq!(&output[..2], &[0, 1]);
            }
        }
    }

    #[test]
    fn test_delimited_field_boundaries() {
        for (wire, required, expected, rest) in [
            (b"".as_slice(), true, Err(Error::Invalid), b"".as_slice()),
            (b"AB", true, Err(Error::Invalid), b"AB"),
            (b"A|B", true, Ok(b"A".as_slice()), b"B"),
            (b"|B", true, Ok(b""), b"B"),
            (b"A|", true, Ok(b"A"), b""),
            (b"A|B", false, Ok(b"A|B"), b""),
            (b"", false, Ok(b""), b""),
        ] {
            let mut input = wire;
            assert_eq!(decode_delimited_field(&mut input, b'|', required), expected);
            assert_eq!(input, rest);
        }
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
            DelimitedSlotsFmt::encode_field(&mut out, scratch.as_mut_slice(), &value).unwrap();
            total - out.len()
        };
        assert_eq!(&output[..used], b"ABCD\\X  \\WXYZ");

        let mut input = &output[..used];
        let decoded = DelimitedSlotsFmt::decode_field(&mut input, &mut scratch.as_mut_slice()).unwrap();
        assert!(input.is_empty());
        assert_eq!(decoded, value);

        let separator_inside = DelimitedSlots {
            first: "AB\\D".into(),
            ..value
        };
        let result =
            std::panic::catch_unwind(|| DelimitedSlotsFmt::encode_field(&mut &mut [0u8; 32][..], &mut [0u8; 32], &separator_inside));
        assert_eq!(result.is_err(), cfg!(debug_assertions));

        let mut invalid = b"ABCD\\X  ".as_slice();
        assert_eq!(
            error_kind(DelimitedSlotsFmt::decode_field(&mut invalid, &mut scratch.as_mut_slice())),
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
            ConcatNoDefaultFmt::encode_field(&mut out, scratch.as_mut_slice(), &concat).unwrap();
            total - out.len()
        };
        assert_eq!(&output[..concat_used], b"HEAD12ABCD");
        let mut input = &output[..concat_used];
        assert_eq!(
            ConcatNoDefaultFmt::decode_field(&mut input, &mut scratch.as_mut_slice()).unwrap(),
            concat
        );
        assert!(input.is_empty());

        let delimited_used = {
            let total = output.len();
            let mut out = output.as_mut_slice();
            DelimitedNoDefaultFmt::encode_field(&mut out, scratch.as_mut_slice(), &delimited).unwrap();
            total - out.len()
        };
        assert_eq!(&output[..delimited_used], b"HEAD|12|ABCD");
        let mut input = &output[..delimited_used];
        assert_eq!(
            DelimitedNoDefaultFmt::decode_field(&mut input, &mut scratch.as_mut_slice()).unwrap(),
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
            OptionalNoDefaultAbsentFmt::encode_field(&mut out, scratch.as_mut_slice(), &None::<NoDefaultAbsentValue>).unwrap();
            total - out.len()
        };
        assert_eq!(&output[..optional_none], b"    ");
        let mut input = &output[..optional_none];
        assert_eq!(
            OptionalNoDefaultAbsentFmt::decode_field(&mut input, &mut scratch.as_mut_slice()).unwrap(),
            None::<NoDefaultAbsentValue>
        );
        assert!(input.is_empty());

        let optional_some = Some(NoDefaultAbsentValue("ABCD".into()));
        let optional_some_used = {
            let total = output.len();
            let mut out = output.as_mut_slice();
            OptionalNoDefaultAbsentFmt::encode_field(&mut out, scratch.as_mut_slice(), &optional_some).unwrap();
            total - out.len()
        };
        assert_eq!(&output[..optional_some_used], b"ABCD");
        let mut input = &output[..optional_some_used];
        assert_eq!(
            OptionalNoDefaultAbsentFmt::decode_field(&mut input, &mut scratch.as_mut_slice()).unwrap(),
            optional_some
        );
        assert!(input.is_empty());

        let values = vec![NoDefaultAbsentValue("ABCD".into())];
        let repeated_fill_used = {
            let total = output.len();
            let mut out = output.as_mut_slice();
            RepeatedNoDefaultByteFillFmt::encode_field(&mut out, scratch.as_mut_slice(), &values).unwrap();
            total - out.len()
        };
        assert_eq!(&output[..repeated_fill_used], b"\xF1ABCD    ");
        let mut input = &output[..repeated_fill_used];
        assert_eq!(
            <RepeatedNoDefaultByteFillFmt as FieldDecode<'_, Vec<NoDefaultAbsentValue>>>::decode_field(
                &mut input,
                &mut scratch.as_mut_slice()
            )
            .unwrap(),
            values
        );
        assert!(input.is_empty());

        let repeated_absent_used = {
            let total = output.len();
            let mut out = output.as_mut_slice();
            RepeatedNoDefaultAbsentFmt::encode_field(&mut out, scratch.as_mut_slice(), &values).unwrap();
            total - out.len()
        };
        assert_eq!(&output[..repeated_absent_used], b"\xF1ABCDNONE");
        let mut input = &output[..repeated_absent_used];
        assert_eq!(
            <RepeatedNoDefaultAbsentFmt as FieldDecode<'_, Vec<NoDefaultAbsentValue>>>::decode_field(
                &mut input,
                &mut scratch.as_mut_slice()
            )
            .unwrap(),
            values
        );
        assert!(input.is_empty());

        let literal_none = {
            let total = output.len();
            let mut out = output.as_mut_slice();
            OptionalNoDefaultLiteralAbsentFmt::encode_field(&mut out, scratch.as_mut_slice(), &None::<NoDefaultAbsentValue>).unwrap();
            total - out.len()
        };
        assert_eq!(&output[..literal_none], b"NONE");
        let mut input = &output[..literal_none];
        assert_eq!(
            OptionalNoDefaultLiteralAbsentFmt::decode_field(&mut input, &mut scratch.as_mut_slice()).unwrap(),
            None::<NoDefaultAbsentValue>
        );
        assert!(input.is_empty());
    }

    #[test]
    fn test_struct_error_field_paths() {
        let mut output = [0u8; 32];
        let mut scratch = [0u8; 32];

        let invalid = DelimitedSlots {
            first: "AB\u{e9}".into(),
            second: "X".into(),
            third: "WXYZ".into(),
        };
        let mut out = output.as_mut_slice();
        let error = DelimitedSlotsFmt::encode_field(&mut out, scratch.as_mut_slice(), &invalid).unwrap_err();
        assert_eq!(error.kind, Error::Invalid);
        assert_eq!(error.path(), [crate::PathSegment::Field("first")]);
        assert!(!error.is_truncated());

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
        let error = NestedConcatFmt::encode_field(&mut out, scratch.as_mut_slice(), &nested).unwrap_err();
        assert_eq!(error.kind, Error::InvalidValueLength);
        assert_eq!(error.path(), [crate::PathSegment::Field("inner"), crate::PathSegment::Field("c")]);
        assert!(!error.is_truncated());
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
                NestedConcatFmt::encode_field(&mut out_ptr, scratch.as_mut_slice(), &value).map(|_| total - out_ptr.len())
            };
            if let Some(expected) = expected {
                let encoded = encoded.unwrap();
                assert_eq!(&output[..encoded], expected);
                let mut input = &output[..encoded];
                let mut decode_scratch = [0u8; 32];
                let decoded = NestedConcatFmt::decode_field(&mut input, &mut decode_scratch.as_mut_slice()).unwrap();
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
            F: FieldEncode<T>,
        {
            let total = output.len();
            let mut out = output;
            F::encode_field(&mut out, scratch, value).map(|_| total - out.len()).unwrap()
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
        let scratch_start = decode_scratch.as_ptr();
        let decoded = BorrowedConcatFmt::decode_field(&mut input, &mut decode_scratch.as_mut_slice()).unwrap();
        assert_eq!(decoded, concat);
        assert!(input.is_empty());
        assert_eq!(decoded.ascii.as_ptr(), output.as_ptr());
        assert_eq!(decoded.ebcdic.as_ptr(), scratch_start);
        let mut json = [0; 64];
        let mut writer = &mut json[..];
        serde_json::to_writer(&mut writer, &decoded).unwrap();
        let json_len = 64 - writer.len();
        assert_eq!(&json[..json_len], br#"{"ascii":"ABCD","ebcdic":"WXYZ"}"#);

        let used = roundtrip::<_, BorrowedDelimitedFmt>(&delimited, output.as_mut_slice(), scratch.as_mut_slice());
        let mut input = &output[..used];
        let decoded = BorrowedDelimitedFmt::decode_field(&mut input, &mut decode_scratch.as_mut_slice()).unwrap();
        assert_eq!(decoded, delimited);
        assert!(input.is_empty());

        let used = roundtrip::<_, BorrowedBitmapFmt>(&bitmap, output.as_mut_slice(), scratch.as_mut_slice());
        let mut input = &output[..used];
        let decoded = BorrowedBitmapFmt::decode_field(&mut input, &mut decode_scratch.as_mut_slice()).unwrap();
        assert_eq!(decoded, bitmap);
        assert!(input.is_empty());

        let used = roundtrip::<_, BorrowedTlvFmt>(&tlv, output.as_mut_slice(), scratch.as_mut_slice());
        let mut input = &output[..used];
        let decoded = BorrowedTlvFmt::decode_field(&mut input, &mut decode_scratch.as_mut_slice()).unwrap();
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
            FramedConcatFmt::encode_field(&mut out_ptr, scratch.as_mut_slice(), &value).map(|_| total - out_ptr.len())
        }
        .unwrap();
        assert_eq!(&output[..encoded], b"121212345678ABCD34");
        let mut input = &output[..encoded];
        let mut decode_scratch = [0u8; 64];
        let decoded = FramedConcatFmt::decode_field(&mut input, &mut decode_scratch.as_mut_slice()).unwrap();
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
            FramedHexConcatFmt::encode_field(&mut out_ptr, scratch.as_mut_slice(), &value).map(|_| total - out_ptr.len())
        }
        .unwrap();
        assert_eq!(&output[..encoded], b"123424313233343536373841424344");
        let mut input = &output[..encoded];
        let mut decode_scratch = [0u8; 64];
        let decoded = FramedHexConcatFmt::decode_field(&mut input, &mut decode_scratch.as_mut_slice()).unwrap();
        assert_eq!(decoded, value);
        assert!(input.is_empty());
    }

    #[test]
    fn staging_writes_only_the_message_to_output() {
        // The inner frame stages while the outer one is staging.
        type Nested = Frame<Field<Binary<0, 99>, AsciiLength<2>>, FramedConcatFmt>;
        let value = FramedConcat {
            head: "12".into(),
            tail: "34".into(),
            inner: FixedTail {
                a: "123456".into(),
                b: "78".into(),
                c: "ABCD".into(),
            },
        };
        let mut output = [0xEE; 64];
        let used = crate::encode::<Nested, _>(&mut output, &mut [0; 48], &value).unwrap();
        assert_eq!(&output[..used], b"18121212345678ABCD34");
        assert!(output[used..].iter().all(|&byte| byte == 0xEE));
        let mut exact = [0; 20];
        assert_eq!(crate::encode::<Nested, _>(&mut exact, &mut [0; 48], &value), Ok(20));
        // The inner frame's 12 bytes need half of the back half: 48 bytes in all.
        let overflow = crate::encode::<Nested, _>(&mut output, &mut [0; 46], &value).unwrap_err();
        assert_eq!(overflow.kind, Error::BufferOverflow);
    }

    fn check_trailing_length_frame<F>()
    where
        F: FieldEncode<TrailingLengthData> + for<'de> FieldDecode<'de, TrailingLengthData>,
    {
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
            (
                TrailingLengthData {
                    base: "AB".into(),
                    tail1: Some("CDE".into()),
                    tail2: Some("XYZ".into()),
                },
                &b"08ABCDEXYZ"[..],
            ),
        ] {
            let total = output.len();
            let encoded = {
                let mut out = output.as_mut_slice();
                F::encode_field(&mut out, scratch.as_mut_slice(), &value).map(|_| total - out.len())
            }
            .unwrap();
            assert_eq!(&output[..encoded], expected);

            output[encoded..encoded + 4].copy_from_slice(b"TAIL");
            let mut input = &output[..encoded + 4];
            let decoded = F::decode_field(&mut input, &mut scratch.as_mut_slice()).unwrap();
            assert_eq!(decoded, value);
            assert_eq!(input, b"TAIL");
        }

        for (invalid, error) in [
            (&b"01AB      "[..], Error::Invalid),
            (b"03AB      ", Error::Invalid),
            (b"09AB      ", Error::Invalid),
            (b"02ABCDE   ", Error::Invalid),
            (b"05ABCDEXYZ", Error::Invalid),
            (b"02AB     ", Error::UnexpectedEof),
            (b"0", Error::UnexpectedEof),
        ] {
            let mut input = invalid;
            assert_eq!(error_kind(F::decode_field(&mut input, &mut scratch.as_mut_slice())), Err(error));
        }
    }

    #[test]
    fn test_trailing_length_frame_roundtrip_and_validation() {
        check_trailing_length_frame::<TrailingLengthDataFmt>();
    }

    #[test]
    fn test_bitmap_declared_fields() {
        use crate::bitmap::{Bitmap, BitmapLayout};
        #[derive(Debug, PartialEq)]
        struct Record {
            low: Option<String>,
            middle: Option<String>,
            high: Option<String>,
        }
        crate::bitmap_format! { struct Format for Record, BitmapLayout::fixed(3), BitmapBinaryWord {
            3 => low: Option<A2>, 97 => middle: Option<A2>, 192 => high: Option<A2>,
        } }
        crate::__finfmt_bitmap_assert_fields!(BitmapLayout::bits(32), BitmapBinaryWord;
            1 => first: A2, 32 => second: A2);
        let mut output = [0; 32];
        let mut scratch = [0; 16];
        for id in 1..=192 {
            if [3, 97, 192].contains(&id) {
                continue;
            }
            for known in [false, true] {
                let mut bitmap = Bitmap::new();
                bitmap.set(id, true);
                bitmap.set(3, known);
                let mut out = output.as_mut_slice();
                crate::bitmap::encode_bitmap::<BitmapBinaryWord>(&mut out, &mut scratch, &bitmap, BitmapLayout::fixed(3)).unwrap();
                copy_bytes(&mut out, b"XY").unwrap();
                let mut input = &output[..26];
                assert_eq!(
                    error_kind(Format::decode_field(&mut input, &mut &mut scratch[..])),
                    Err(Error::Invalid),
                    "field {id}"
                );
                assert_eq!(input, b"XY");
            }
        }
        for mask in 0..8 {
            let value = Record {
                low: (mask & 1 != 0).then(|| "AB".into()),
                middle: (mask & 2 != 0).then(|| "CD".into()),
                high: (mask & 4 != 0).then(|| "EF".into()),
            };
            let used = {
                let mut out = output.as_mut_slice();
                Format::encode_field(&mut out, &mut scratch, &value).unwrap();
                32 - out.len()
            };
            let mut input = &output[..used];
            assert_eq!(Format::decode_field(&mut input, &mut &mut scratch[..]).unwrap(), value);
            assert!(input.is_empty());
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
            LocalBitmapDataFmt::encode_field(&mut out_ptr, scratch.as_mut_slice(), &value).map(|_| total - out_ptr.len())
        }
        .unwrap();
        assert_eq!(&output[..encoded], b"HEAD\x60\x00\x00\x0012ABCD");
        let mut input = &output[..encoded];
        let mut decode_scratch = [0u8; 32];
        let decoded = LocalBitmapDataFmt::decode_field(&mut input, &mut decode_scratch.as_mut_slice()).unwrap();
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
                BitmapNoDefaultFmt::encode_field(&mut out_ptr, scratch.as_mut_slice(), &value).map(|_| total - out_ptr.len())
            }
            .unwrap();
            let mut input = &output[..used];
            let decoded = BitmapNoDefaultFmt::decode_field(&mut input, &mut scratch.as_mut_slice()).unwrap();
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
                SerdeScalarBitmapDataFmt::encode_field(&mut out_ptr, scratch.as_mut_slice(), &value).map(|_| total - out_ptr.len())
            }
            .unwrap();
            assert_eq!(&output[..encoded], expected);

            let mut input = &output[..encoded];
            let mut decode_scratch = [0u8; 32];
            let decoded = SerdeScalarBitmapDataFmt::decode_field(&mut input, &mut decode_scratch.as_mut_slice()).unwrap();
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
                ManualScalarBitmapDataFmt::encode_field(&mut out_ptr, scratch.as_mut_slice(), &value).map(|_| total - out_ptr.len())
            }
            .unwrap();
            assert_eq!(&output[..encoded], expected);

            let mut input = &output[..encoded];
            let mut decode_scratch = [0u8; 32];
            let decoded = ManualScalarBitmapDataFmt::decode_field(&mut input, &mut decode_scratch.as_mut_slice()).unwrap();
            assert_eq!(decoded, value);
            assert!(input.is_empty());
        }
    }

    fn assert_field_syntax_roundtrip<F>(value: &FieldSyntaxRecord)
    where
        F: FieldEncode<FieldSyntaxRecord> + for<'de> FieldDecode<'de, FieldSyntaxRecord>,
    {
        let mut output = [0u8; 128];
        let mut scratch = [0u8; 128];
        let total = output.len();
        let used = {
            let mut out_ptr = output.as_mut_slice();
            F::encode_field(&mut out_ptr, scratch.as_mut_slice(), value).map(|_| total - out_ptr.len())
        }
        .unwrap();
        let mut input = &output[..used];
        let mut decode_scratch = [0u8; 128];
        let decoded: FieldSyntaxRecord = F::decode_field(&mut input, &mut decode_scratch.as_mut_slice()).unwrap();
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
                FieldSyntaxConcatFmt::encode_field(&mut out_ptr, scratch.as_mut_slice(), &tail_gap)
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
            <A4 as FieldEncode<&str>>::encode_field(&mut out_ptr, scratch.as_mut_slice(), &value).map(|_| total - out_ptr.len())
        }
        .unwrap();
        assert_eq!(&output[..encoded], b"ABCD");

        let mut input = &output[..encoded];
        let decoded = <A4 as FieldDecode<'_, &str>>::decode_field(&mut input, &mut scratch.as_mut_slice()).unwrap();
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
            SerdeDualStanDataFmt::encode_field(&mut out_ptr, serde_scratch.as_mut_slice(), &serde_value).map(|_| total - out_ptr.len())
        }
        .unwrap();
        assert_eq!(&serde_output[..serde_len], &b"\x00\x20\x00\x00\x00\x00\x00\x0017"[..]);

        let mut manual_output = [0u8; 16];
        let mut manual_scratch = [0u8; 16];
        let manual_len = {
            let total = manual_output.len();
            let mut out_ptr = manual_output.as_mut_slice();
            ManualDualStanDataFmt::encode_field(&mut out_ptr, manual_scratch.as_mut_slice(), &manual_value).map(|_| total - out_ptr.len())
        }
        .unwrap();
        assert_eq!(&manual_output[..manual_len], &b"\x00\x20\x00\x00\x00\x00\x00\x0027"[..]);

        let mut serde_input = &serde_output[..serde_len];
        let mut serde_decode_scratch = [0u8; 16];
        let serde_decoded = SerdeDualStanDataFmt::decode_field(&mut serde_input, &mut serde_decode_scratch.as_mut_slice()).unwrap();
        assert_eq!(serde_decoded, serde_value);
        assert!(serde_input.is_empty());

        let mut manual_input = &manual_output[..manual_len];
        let mut manual_decode_scratch = [0u8; 16];
        let manual_decoded = ManualDualStanDataFmt::decode_field(&mut manual_input, &mut manual_decode_scratch.as_mut_slice()).unwrap();
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
                BoolScalarDataFmt::encode_field(&mut out_ptr, scratch.as_mut_slice(), &BoolScalarData { value: true })
            }),
            error_kind({
                let mut out_ptr = output.as_mut_slice();
                BytesScalarDataFmt::encode_field(&mut out_ptr, scratch.as_mut_slice(), &BytesScalarData { value: vec![0x31, 0x32] })
            }),
            error_kind({
                let mut out_ptr = output.as_mut_slice();
                StructScalarDataFmt::encode_field(
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
            error_kind(BoolScalarDataFmt::decode_field(&mut b"12".as_slice(), &mut scratch.as_mut_slice())).map(|_| ()),
            error_kind(BytesScalarDataFmt::decode_field(
                &mut b"ABCD".as_slice(),
                &mut scratch.as_mut_slice(),
            ))
            .map(|_| ()),
            error_kind(StructScalarDataFmt::decode_field(
                &mut b"ABCD".as_slice(),
                &mut scratch.as_mut_slice(),
            ))
            .map(|_| ()),
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
                VariantDataFmt::encode_field(&mut out_ptr, scratch.as_mut_slice(), &value).map(|_| total - out_ptr.len())
            }
            .unwrap();
            assert_eq!(&output[..encoded], expected);
            let mut input = &output[..encoded];
            let mut decode_scratch = [0u8; 32];
            let decoded = VariantDataFmt::decode_field(&mut input, &mut decode_scratch.as_mut_slice()).unwrap();
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
            RetainedVariantRecordFmt::encode_field(&mut out, scratch.as_mut_slice(), &value).map(|_| total - out.len())
        }
        .unwrap();
        assert_eq!(&output[..used], b"AXBBWXYZDONE");

        let mut input = &output[..used];
        let mut decode_scratch = [0u8; 32];
        let decoded = RetainedVariantRecordFmt::decode_field(&mut input, &mut decode_scratch.as_mut_slice()).unwrap();
        assert_eq!(decoded, value);
        assert!(input.is_empty());

        let used = {
            let mut out = output.as_mut_slice();
            DelimitedRetainedVariantRecordFmt::encode_field(&mut out, scratch.as_mut_slice(), &value).map(|_| total - out.len())
        }
        .unwrap();
        assert_eq!(&output[..used], b"AXBB|WXYZ|DONE");

        let mut input = &output[..used];
        let decoded = DelimitedRetainedVariantRecordFmt::decode_field(&mut input, &mut decode_scratch.as_mut_slice()).unwrap();
        assert_eq!(decoded, value);
        assert!(input.is_empty());

        let selector = VariantSelector { code: "AXBB".into() };
        let borrowed = BorrowedVariantData::B(BorrowedVariantTail { tail: "WXYZ" });
        let used = {
            let mut out = output.as_mut_slice();
            let scratch_ptr = scratch.as_mut_slice();
            BorrowedRetainedVariantDataFmt::encode_with(&mut out, scratch_ptr, &selector, &borrowed).map(|_| total - out.len())
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
            error_kind(RetainedVariantRecordFmt::encode_field(&mut out, scratch.as_mut_slice(), &mismatch)),
            Err(Error::Invalid)
        );

        let mut unknown = &b"AXZZWXYZDONE"[..];
        assert_eq!(
            error_kind(RetainedVariantRecordFmt::decode_field(&mut unknown, &mut scratch.as_mut_slice())),
            Err(Error::Invalid)
        );

        let with_separator = RetainedVariantRecord {
            selector: VariantSelector { code: "AXBB".into() },
            body: RetainedVariantData::B(VariantBTail { tail: "WX|Z".into() }),
            suffix: "DONE".into(),
        };
        let separator_inside = std::panic::catch_unwind(|| {
            DelimitedRetainedVariantRecordFmt::encode_field(&mut &mut [0u8; 32][..], &mut [0u8; 32], &with_separator)
        });
        assert_eq!(separator_inside.is_err(), cfg!(debug_assertions));
    }

    #[test]
    fn test_borrowed_tagged_format_roundtrip() {
        let mut output = [0u8; 32];
        let mut scratch = [0u8; 32];

        let value = BorrowedVariantData::B(BorrowedVariantTail { tail: "WXYZ" });
        let used = {
            let total = output.len();
            let mut out = output.as_mut_slice();
            BorrowedVariantDataFmt::encode_field(&mut out, scratch.as_mut_slice(), &value).map(|_| total - out.len())
        }
        .unwrap();
        assert_eq!(&output[..used], b"AXBBWXYZ");
        let mut input = &output[..used];
        let decoded = BorrowedVariantDataFmt::decode_field(&mut input, &mut scratch.as_mut_slice()).unwrap();
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
                Auth1100Fmt::encode_field(&mut out_ptr, scratch.as_mut_slice(), &value).unwrap();
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
            let decoded = Auth1100Fmt::decode_field(&mut input, &mut decode_scratch.as_mut_slice()).unwrap();
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
            TlvDataFmt::encode_field(&mut out_ptr, scratch.as_mut_slice(), &value).unwrap();
            total - out_ptr.len()
        };

        assert_eq!(&output[..used], b"\x59\x04ABCD\xDF\x23\x0C12345678WXYZ");

        let with_unknown = b"\x59\x04ABCD\x9F\x01\x01\xFF\xDF\x23\x0C12345678WXYZ";
        let mut input = &with_unknown[..];
        let mut decode_scratch = [0u8; 64];
        assert_eq!(
            error_kind(TlvDataFmt::decode_field(&mut input, &mut decode_scratch.as_mut_slice())),
            Err(Error::Invalid)
        );
    }

    #[test]
    fn test_ber_tlv_duplicate_rejected_and_nested_bitmap_roundtrip() {
        let mut dup = &b"\x59\x04ABCD\x59\x04WXYZ"[..];
        let mut scratch = [0u8; 32];
        assert_eq!(
            error_kind(TlvDataFmt::decode_field(&mut dup, &mut scratch.as_mut_slice())),
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
            WithBitmapTlvFmt::encode_field(&mut out_ptr, encode_scratch.as_mut_slice(), &value).unwrap();
            total - out_ptr.len()
        };

        assert_eq!(&output[..8], b"\x20\x00\x00\x00\x00\x01\x00\x00");
        assert_eq!(&output[8..14], b"123456");
        assert_eq!(&output[14..used], b"\x59\x04ABCD\xDF\x23\x0C12345678WXYZ");

        let mut input = &output[..used];
        let mut decode_scratch = [0u8; 128];
        let decoded = WithBitmapTlvFmt::decode_field(&mut input, &mut decode_scratch.as_mut_slice()).unwrap();
        assert_eq!(decoded, value);
        assert!(input.is_empty());
    }

    #[test]
    fn test_ber_tlv_extras_roundtrip_and_flattened_json() {
        let bytes = b"\x59\x04ABCD\x9F\x02\x02\x12\x34";
        let mut input = &bytes[..];
        let mut scratch = [0u8; 64];
        let decoded = TlvWithExtrasFmt::decode_field(&mut input, &mut scratch.as_mut_slice()).unwrap();
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
            TlvWithExtrasFmt::encode_field(&mut out_ptr, scratch.as_mut_slice(), &decoded).unwrap();
            total - out_ptr.len()
        };
        assert_eq!(&output[..used], bytes);
    }

    #[test]
    fn test_named_ber_padding_is_local_and_opt_in() {
        crate::ber_tlv_format! { struct Padded for TlvData, allow_zero_padding = true {
            "59" => t59_code: A4, "DF23" => tdf23_tail: Option<FixedTailFmt>,
        } }
        crate::ber_tlv_format! { struct Strict for TlvData, allow_zero_padding = false {
            "59" => t59_code: A4, "DF23" => tdf23_tail: Option<FixedTailFmt>,
        } }
        crate::ber_tlv_format! { struct PaddedExtras for TlvWithExtras, allow_zero_padding = true {
            extras: extras, "59" => t59_code: A4,
        } }
        #[derive(Debug, PartialEq, Serialize)]
        struct Borrowed<'a> {
            code: &'a str,
        }
        #[derive(Debug, PartialEq, Serialize)]
        struct BorrowedExtras<'a> {
            code: &'a str,
            extras: BTreeMap<String, String>,
        }
        crate::ber_tlv_format! { struct BorrowedFmt for<'a> Borrowed<'a>, allow_zero_padding = true { "59" => code: A4, } }
        crate::ber_tlv_format! { struct BorrowedExtrasFmt for<'a> BorrowedExtras<'a>, allow_zero_padding = true { extras: extras, "59" => code: A4, } }
        for bytes in [b"\0\x59\x04ABCD".as_slice(), b"\x59\x04ABCD\0", b"\0\x59\x04ABCD\0\0"] {
            let expected = TlvData {
                t59_code: "ABCD".into(),
                tdf23_tail: None,
            };
            assert_eq!(Padded::decode_field(&mut &*bytes, &mut &mut [0; 64][..]).unwrap(), expected);
            assert_eq!(
                error_kind(Strict::decode_field(&mut &*bytes, &mut &mut [0; 64][..])),
                Err(Error::Invalid)
            );
            assert_eq!(
                error_kind(TlvDataFmt::decode_field(&mut &*bytes, &mut &mut [0; 64][..])),
                Err(Error::Invalid)
            );
            assert_eq!(
                BorrowedFmt::decode_field(&mut &*bytes, &mut &mut [][..]).unwrap(),
                Borrowed { code: "ABCD" }
            );
            let mut output = [0; 16];
            let mut out = output.as_mut_slice();
            Padded::encode_field(&mut out, &mut [0; 8], &expected).unwrap();
            let used = 16 - out.len();
            assert_eq!(&output[..used], b"\x59\x04ABCD");
        }
        let wire = b"\0\x59\x04ABCD\0\xFF\x01\x02\0\xFF\0";
        let extras = BTreeMap::from([("tFF01_unknown".into(), "00FF".into())]);
        assert_eq!(
            PaddedExtras::decode_field(&mut wire.as_slice(), &mut &mut [0; 64][..]).unwrap(),
            TlvWithExtras {
                t59_code: "ABCD".into(),
                extras: extras.clone()
            }
        );
        assert_eq!(
            BorrowedExtrasFmt::decode_field(&mut wire.as_slice(), &mut &mut [0; 64][..]).unwrap(),
            BorrowedExtras { code: "ABCD", extras }
        );
        assert_eq!(
            error_kind(Padded::decode_field(&mut b"\0\0".as_slice(), &mut &mut [][..])),
            Err(Error::Invalid)
        );

        #[derive(Debug, PartialEq, Serialize)]
        struct Outer {
            inner: TlvData,
        }
        crate::ber_tlv_format! { struct StrictInner for Outer, allow_zero_padding = true { "E1" => inner: TlvDataFmt, } }
        crate::ber_tlv_format! { struct PaddedInner for Outer, allow_zero_padding = true { "E1" => inner: Padded, } }
        let nested = b"\0\xE1\x07\0\x59\x04ABCD\0";
        assert_eq!(
            error_kind(StrictInner::decode_field(&mut nested.as_slice(), &mut &mut [0; 64][..])),
            Err(Error::Invalid)
        );
        assert_eq!(
            PaddedInner::decode_field(&mut nested.as_slice(), &mut &mut [0; 64][..]).unwrap(),
            Outer {
                inner: TlvData {
                    t59_code: "ABCD".into(),
                    tdf23_tail: None
                }
            }
        );
    }

    #[test]
    fn test_ber_tlv_extras_reject_declared_tags() {
        #[derive(Serialize, Deserialize)]
        struct Record {
            code: Option<String>,
            #[serde(flatten)]
            extras: BTreeMap<String, String>,
        }
        #[derive(Serialize)]
        struct BorrowedRecord<'a> {
            code: Option<&'a str>,
            extras: BTreeMap<String, String>,
        }
        crate::ber_tlv_format! { struct Fmt for Record { extras: extras, "59" => code: Option<A4>, } }
        crate::ber_tlv_format! { struct BorrowedFmt for<'a> BorrowedRecord<'a> { extras: extras, "59" => code: Option<A4>, } }

        for json in [r#"{"code":"ABCD","t59_unknown":"5758595A"}"#, r#"{"t59_unknown":"5758595A"}"#] {
            let value: Record = serde_json::from_str(json).unwrap();
            let borrowed = BorrowedRecord {
                code: value.code.as_deref(),
                extras: value.extras.clone(),
            };
            let expected = CompositeError::from(Error::Invalid).with_field("extras");
            // Values are staged in scratch before their head is written.
            assert_eq!(Fmt::encode_field(&mut [0; 32].as_mut_slice(), &mut [0; 8], &value), Err(expected));
            assert_eq!(
                BorrowedFmt::encode_field(&mut [0; 32].as_mut_slice(), &mut [0; 8], &borrowed),
                Err(expected)
            );
        }
    }

    #[test]
    fn test_ber_tlv_extras_repeated_unknown_tag_keeps_the_last_in_a_map() {
        let bytes = b"\x59\x04ABCD\x9F\x02\x01\x01\x9F\x02\x01\x02";
        let mut scratch = [0u8; 64];
        let decoded = TlvWithExtrasFmt::decode_field(&mut &bytes[..], &mut scratch.as_mut_slice()).unwrap();
        assert_eq!(decoded.extras, BTreeMap::from([("t9F02_unknown".to_owned(), "02".to_owned())]));
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
            error_kind(TlvWithExtrasFmt::encode_field(&mut out_ptr, scratch.as_mut_slice(), &invalid_key)),
            Err(Error::Invalid)
        );
        let mut out_ptr = output.as_mut_slice();
        assert_eq!(
            error_kind(TlvWithExtrasFmt::encode_field(&mut out_ptr, scratch.as_mut_slice(), &invalid_value)),
            Err(Error::Invalid)
        );
    }

    #[test]
    fn test_ber_tlv_list_roundtrip_preserves_order_and_duplicates() {
        type TlvListFmt = BerTlvList;

        let value = vec![
            ("59".to_owned(), "ABCD".to_owned()),
            ("9F02".to_owned(), "1234".to_owned()),
            ("59".to_owned(), "00FF".to_owned()),
        ];
        let mut output = [0u8; 64];
        let mut scratch = [0u8; 64];
        let used = {
            let total = output.len();
            let mut out = output.as_mut_slice();
            TlvListFmt::encode_field(&mut out, scratch.as_mut_slice(), &value).map(|_| total - out.len())
        }
        .unwrap();

        let mut input = &output[..used];
        let decoded: Vec<(String, String)> = TlvListFmt::decode_field(&mut input, &mut scratch.as_mut_slice()).unwrap();
        assert_eq!(decoded, value);
        assert!(input.is_empty());
    }

    #[test]
    fn ber_tlv_text_reuses_decode_scratch_per_entry() {
        // Ten entries need 210 bytes of key and hex text in total, but each is
        // parsed into an owned value before the next, so one entry's worth is enough.
        let wire: Vec<u8> = (0..10u8).flat_map(|i| [0x9F, i + 1, 0x04, 0xDE, 0xAD, 0xBE, 0xEF]).collect();
        let mut scratch = [0u8; 21];
        let list =
            <BerTlvList as FieldDecode<'_, Vec<(String, String)>>>::decode_field(&mut wire.as_slice(), &mut &mut scratch[..]).unwrap();
        assert_eq!(list.len(), 10);
        let map =
            <BerTlvList as FieldDecode<'_, BTreeMap<String, String>>>::decode_field(&mut wire.as_slice(), &mut &mut scratch[..]).unwrap();
        assert_eq!(map.len(), 10);
        let with_known = [&b"\x59\x04ABCD"[..], &wire].concat();
        let extras = TlvWithExtrasFmt::decode_field(&mut with_known.as_slice(), &mut &mut scratch[..]).unwrap();
        assert_eq!(extras.extras.len(), 10);
    }

    #[test]
    fn test_ber_tlv_list_supports_map_and_newtype_wrappers() {
        #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(transparent)]
        struct TlvSeqWrapper(Vec<(String, String)>);

        type TlvMapFmt = BerTlvList;
        type TlvSeqWrapperFmt = BerTlvList;

        let map = BTreeMap::from([("59".to_owned(), "ABCD".to_owned()), ("9F02".to_owned(), "1234".to_owned())]);
        let wrapper = TlvSeqWrapper(vec![
            ("59".to_owned(), "ABCD".to_owned()),
            ("9F02".to_owned(), "1234".to_owned()),
            ("59".to_owned(), "00FF".to_owned()),
        ]);
        let map_bytes = b"\x59\x02\xAB\xCD\x9F\x02\x02\x12\x34";
        let wrapper_bytes = b"\x59\x02\xAB\xCD\x9F\x02\x02\x12\x34\x59\x02\x00\xFF";
        let mut scratch = [0u8; 64];

        let mut output = [0u8; 64];
        let used = {
            let total = output.len();
            let mut out = output.as_mut_slice();
            TlvMapFmt::encode_field(&mut out, scratch.as_mut_slice(), &map).map(|_| total - out.len())
        }
        .unwrap();
        assert_eq!(&output[..used], map_bytes);
        let mut input = map_bytes.as_slice();
        let decoded: BTreeMap<String, String> = TlvMapFmt::decode_field(&mut input, &mut scratch.as_mut_slice()).unwrap();
        assert_eq!(decoded, map);
        assert!(input.is_empty());

        let used = {
            let total = output.len();
            let mut out = output.as_mut_slice();
            TlvSeqWrapperFmt::encode_field(&mut out, scratch.as_mut_slice(), &wrapper).map(|_| total - out.len())
        }
        .unwrap();
        assert_eq!(&output[..used], wrapper_bytes);
        let mut input = wrapper_bytes.as_slice();
        let decoded: TlvSeqWrapper = TlvSeqWrapperFmt::decode_field(&mut input, &mut scratch.as_mut_slice()).unwrap();
        assert_eq!(decoded, wrapper);
        assert!(input.is_empty());
    }

    #[test]
    fn test_ber_tlv_list_invalid_key_or_value_rejected_on_encode() {
        type TlvListFmt = BerTlvList;

        let invalid_key = vec![("bad".to_owned(), "1234".to_owned())];
        let invalid_value = vec![("9F02".to_owned(), "12fg".to_owned())];
        let mut output = [0u8; 64];
        let mut scratch = [0u8; 64];
        let mut out = output.as_mut_slice();
        assert_eq!(
            error_kind(TlvListFmt::encode_field(&mut out, scratch.as_mut_slice(), &invalid_key)),
            Err(Error::Invalid)
        );
        let mut out = output.as_mut_slice();
        assert_eq!(
            error_kind(TlvListFmt::encode_field(&mut out, scratch.as_mut_slice(), &invalid_value)),
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
                TlvNoDefaultFmt::encode_field(&mut out, scratch.as_mut_slice(), &value).map(|_| total - out.len())
            }
            .unwrap();
            let mut input = &output[..used];
            assert_eq!(
                TlvNoDefaultFmt::decode_field(&mut input, &mut scratch.as_mut_slice()).unwrap(),
                value
            );
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
            TlvWithExtrasNoDefaultFmt::encode_field(&mut out, scratch.as_mut_slice(), &value).map(|_| total - out.len())
        }
        .unwrap();
        let mut input = &output[..used];
        assert_eq!(
            TlvWithExtrasNoDefaultFmt::decode_field(&mut input, &mut scratch.as_mut_slice()).unwrap(),
            value
        );
        assert!(input.is_empty());
    }

    #[test]
    fn test_union_format_decode_and_encode() {
        let mut scratch = [0u8; 16];

        let mut input = b"1234".as_slice();
        assert_eq!(
            UnionValueFmt::decode_field(&mut input, &mut scratch.as_mut_slice()),
            Ok(UnionValue::Known("1234".into()))
        );
        assert!(input.is_empty());

        let mut input = b"ABCD".as_slice();
        assert_eq!(
            UnionValueFmt::decode_field(&mut input, &mut scratch.as_mut_slice()),
            Ok(UnionValue::Alpha("ABCD".into()))
        );
        assert!(input.is_empty());

        let mut input = b"12AB".as_slice();
        let mut exact_scratch = [0u8; 4];
        assert_eq!(
            UnionValueFmt::decode_field(&mut input, &mut exact_scratch.as_mut_slice()),
            Ok(UnionValue::Unknown("12AB".into()))
        );
        assert!(input.is_empty());

        let mut input = b"12".as_slice();
        assert_eq!(
            UnionValueFmt::decode_field(&mut input, &mut scratch.as_mut_slice()),
            Ok(UnionValue::Unknown("12".into()))
        );
        assert!(input.is_empty());

        let mut input = b"12345".as_slice();
        assert_eq!(
            RestUnionValueFmt::decode_field(&mut input, &mut scratch.as_mut_slice()),
            Ok(UnionValue::Unknown("12345".into()))
        );
        assert!(input.is_empty());

        let mut input = b"12".as_slice();
        let error = ShortUnionValueFmt::decode_field(&mut input, &mut scratch.as_mut_slice()).unwrap_err();
        assert_eq!(error.kind, Error::UnexpectedEof);
        assert!(error.path().is_empty());
        assert_eq!(input, b"12");

        let mut output = [0u8; 16];
        let used = {
            let total = output.len();
            let mut out = output.as_mut_slice();
            UnionValueFmt::encode_field(&mut out, scratch.as_mut_slice(), &UnionValue::Known("1234".into())).unwrap();
            total - out.len()
        };
        assert_eq!(&output[..used], b"1234");

        let used = {
            let total = output.len();
            let mut out = output.as_mut_slice();
            UnionValueFmt::encode_field(&mut out, scratch.as_mut_slice(), &UnionValue::Unknown("ABCD".into())).unwrap();
            total - out.len()
        };
        assert_eq!(&output[..used], b"ABCD");

        let mut input = b"1234".as_slice();
        assert_eq!(
            BorrowedUnionValueFmt::decode_field(&mut input, &mut scratch.as_mut_slice()),
            Ok(BorrowedUnionValue::Known("1234"))
        );
        assert!(input.is_empty());

        let mut input = b"\xC1\xC2\xC3\xC4".as_slice();
        assert_eq!(
            BorrowedUnionValueFmt::decode_field(&mut input, &mut scratch.as_mut_slice()),
            Ok(BorrowedUnionValue::Unknown("ABCD"))
        );
        assert!(input.is_empty());

        let used = {
            let total = output.len();
            let mut out = output.as_mut_slice();
            BorrowedUnionValueFmt::encode_field(&mut out, scratch.as_mut_slice(), &BorrowedUnionValue::Unknown("ABCD")).unwrap();
            total - out.len()
        };
        assert_eq!(&output[..used], b"\xC1\xC2\xC3\xC4");
    }
}
mod enum_macros;
mod wrappers;

#[cfg(feature = "serde")]
pub use scalar_serde::SerdeScalar;
#[cfg(feature = "serde")]
#[doc(hidden)]
pub use scalar_serde::{decode_serde_scalar, encode_serde_scalar};

pub trait ListSeparatorPolicy {
    const BYTE: Option<u8>;
}

pub trait BerTlvExtras {
    /// Encode extras, rejecting tags declared in `known_tags`, even if their fields are absent.
    /// The declarations use uppercase tag hex; pass an empty slice for standalone extras.
    fn encode_unknowns(&self, output: &mut &mut [u8], scratch: &mut [u8], known_tags: &[&str]) -> Result<(), Error>;
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
    copy_bytes(output, &[byte]).map(|_| ())
}

#[inline(always)]
pub fn wrap_composite_error<E: Into<CompositeError>>(error: E, field: &'static str) -> CompositeError {
    error.into().with_field(field)
}

#[inline(always)]
#[doc(hidden)]
pub fn encode_ber_tlv_field<F>(
    output: &mut &mut [u8],
    scratch: &mut [u8],
    tag_hex: &str,
    field: &'static str,
    encode_value: F,
) -> Result<(), CompositeError>
where
    F: FnOnce(&mut &mut [u8], &mut [u8]) -> Result<(), CompositeError>,
{
    let mut tag = [0; crate::primitive::bertlv::MAX_BER_TAG_BYTES];
    let tag = crate::primitive::bertlv::pack_ber_tag_hex(&mut tag, tag_hex);
    // Stage the value, so the head can state its length before it.
    let (value, _) = encode_staged(scratch, encode_value).map_err(|error| wrap_composite_error(error, field))?;
    crate::primitive::bertlv::encode_ber_tlv_head(output, tag, value.len()).map_err(|error| wrap_composite_error(error, field))?;
    copy_bytes(output, value).map_err(|error| wrap_composite_error(error, field))?;
    Ok(())
}

#[inline(always)]
#[doc(hidden)]
pub fn decode_ber_tlv_field<'a, T, D>(
    wire_tag_hex: &[u8],
    tag_hex: &str,
    value_input: &mut &'a [u8],
    scratch: &mut &'a mut [u8],
    field_value: &mut Option<T>,
    field: &'static str,
    decode_value: D,
) -> Result<bool, CompositeError>
where
    D: FnOnce(&mut &'a [u8], &mut &'a mut [u8]) -> Result<T, CompositeError>,
{
    debug_assert!(
        crate::primitive::bertlv::parse_ber_tag_hex(tag_hex).is_ok(),
        "a BER tag literal must be a valid tag in uppercase hex"
    );
    if wire_tag_hex != tag_hex.as_bytes() {
        return Ok(false);
    }
    if field_value.is_some() {
        crate::utils::cold_path();
        return Err(wrap_composite_error(Error::Invalid, field));
    }
    let value = decode_value(value_input, scratch).map_err(|error| wrap_composite_error(error, field))?;
    if !value_input.is_empty() {
        crate::utils::cold_path();
        return Err(wrap_composite_error(Error::Invalid, field));
    }
    *field_value = Some(value);
    Ok(true)
}

#[inline(always)]
#[doc(hidden)]
pub fn should_retry_union(error: Error) -> bool {
    matches!(error, Error::Invalid | Error::UnexpectedEof)
}

/// Decode an owned value with a scratch reborrow the value does not outlive,
/// so a failed trial leaves the arena untouched.
#[inline(always)]
#[doc(hidden)]
pub fn decode_owned_value<'de, T, F>(input: &mut &'de [u8], scratch: &'de mut [u8]) -> Result<T, CompositeError>
where
    F: FieldDecode<'de, T>,
{
    let mut scratch = scratch;
    F::decode_field(input, &mut scratch)
}

/// Encode through `encode`, then, in debug builds only, pass the bytes it wrote
/// to `check`. Release builds encode straight into the output.
#[inline(always)]
pub(crate) fn encode_debug_checked<E, C>(output: &mut &mut [u8], scratch: &mut [u8], encode: E, check: C) -> Result<(), CompositeError>
where
    E: FnOnce(&mut &mut [u8], &mut [u8]) -> Result<(), CompositeError>,
    C: FnOnce(&[u8]),
{
    if !cfg!(debug_assertions) {
        return encode(output, scratch);
    }
    let available = output.len();
    let mut out = &mut **output;
    encode(&mut out, scratch)?;
    let used = available - out.len();
    let (written, rest) = core::mem::take(output).split_at_mut(used);
    check(written);
    *output = rest;
    Ok(())
}

/// Encode one segment. A separator inside it would split the field on decode:
/// the field's check must exclude the separator, which debug builds assert.
#[inline]
fn encode_delimited_segment<E>(output: &mut &mut [u8], scratch: &mut [u8], separator: Option<u8>, encode: E) -> Result<(), CompositeError>
where
    E: FnOnce(&mut &mut [u8], &mut [u8]) -> Result<(), CompositeError>,
{
    encode_debug_checked(output, scratch, encode, |segment| {
        debug_assert!(
            separator.is_none_or(|byte| !segment.contains(&byte)),
            "a delimited field encoded its separator; its check must exclude it"
        );
    })
}

#[inline]
#[doc(hidden)]
pub fn encode_delimited_value<T: ?Sized, F: FieldEncode<T>>(
    output: &mut &mut [u8],
    scratch: &mut [u8],
    value: &T,
    separator: Option<u8>,
) -> Result<(), CompositeError> {
    encode_delimited_segment(output, scratch, separator, |segment_out, nested_scratch| {
        F::encode_field(segment_out, nested_scratch, value)
    })
}

#[inline]
#[doc(hidden)]
pub fn encode_delimited_context<T: ?Sized, C: ?Sized, F: ContextEncode<T, C>>(
    output: &mut &mut [u8],
    scratch: &mut [u8],
    context: &C,
    value: &T,
    separator: Option<u8>,
) -> Result<(), CompositeError> {
    encode_delimited_segment(output, scratch, separator, |segment_out, nested_scratch| {
        F::encode_with(segment_out, nested_scratch, context, value)
    })
}

#[inline(always)]
#[doc(hidden)]
pub fn decode_delimited_field<'a>(input: &mut &'a [u8], separator: u8, expect_separator: bool) -> Result<&'a [u8], Error> {
    if !expect_separator {
        return Ok(core::mem::take(input));
    }
    let mut trial = *input;
    let (segment, terminated) = take_delimited(&mut trial, separator);
    if !terminated {
        crate::utils::cold_path();
        return Err(Error::Invalid);
    }
    *input = trial;
    Ok(segment)
}

#[inline]
fn decode_delimited_segment<'a, T, D>(segment: &'a [u8], scratch: &mut &'a mut [u8], decode: D) -> Result<T, CompositeError>
where
    D: FnOnce(&mut &'a [u8], &mut &'a mut [u8]) -> Result<T, CompositeError>,
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
pub fn decode_delimited_value<'a, T, F: FieldDecode<'a, T>>(segment: &'a [u8], scratch: &mut &'a mut [u8]) -> Result<T, CompositeError> {
    decode_delimited_segment(segment, scratch, |input, scratch| F::decode_field(input, scratch))
}

#[inline]
#[doc(hidden)]
pub fn decode_delimited_context<'a, T, C: ?Sized, F: ContextDecode<'a, T, C>>(
    segment: &'a [u8],
    scratch: &mut &'a mut [u8],
    context: &C,
) -> Result<T, CompositeError> {
    decode_delimited_segment(segment, scratch, |input, scratch| F::decode_with(input, scratch, context))
}

#[inline]
#[doc(hidden)]
pub fn encode_delimited_literal<F: ScalarFmt>(
    output: &mut &mut [u8],
    scratch: &mut [u8],
    expected: &[u8],
    separator: Option<u8>,
) -> Result<(), CompositeError> {
    encode_delimited_segment(output, scratch, separator, |segment_out, scratch| {
        F::encode(segment_out, scratch, expected).map_err(CompositeError::from)
    })
}

#[inline]
#[doc(hidden)]
pub fn decode_delimited_literal<'a, F: ScalarFmt>(
    segment: &'a [u8],
    scratch: &mut &'a mut [u8],
    expected: &[u8],
) -> Result<(), CompositeError> {
    decode_delimited_segment(segment, scratch, |input, scratch| {
        decode_literal::<F>(input, scratch, expected).map_err(CompositeError::from)
    })
}

/// Encode one variant's value. Like [`decode_variant`], a variant body is its
/// own function, so large variants do not inline into each other.
#[inline(never)]
pub(crate) fn encode_variant<T: ?Sized, F: FieldEncode<T>>(
    output: &mut &mut [u8],
    scratch: &mut [u8],
    value: &T,
) -> Result<(), CompositeError> {
    F::encode_field(output, scratch, value)
}

#[inline(never)]
pub(crate) fn decode_variant<'a, T, E, F, W>(input: &mut &'a [u8], scratch: &mut &'a mut [u8], wrap: W) -> Result<E, CompositeError>
where
    F: FieldDecode<'a, T>,
    W: FnOnce(T) -> E,
{
    Ok(wrap(F::decode_field(input, scratch)?))
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

/// Trial-decode a literal in its decoded semantic representation. Input advances
/// only on a match; temporary scratch use is not retained. Decode errors are
/// returned to the caller so a dispatcher can distinguish EOF and resource errors.
#[inline]
pub fn match_literal<'a, F: ScalarFmt>(input: &mut &'a [u8], scratch: &mut &'a mut [u8], expected: &[u8]) -> Result<bool, Error> {
    let source = *input;
    let mut trial = source;
    let mut workspace = &mut **scratch;
    if F::decode(&mut trial, &mut workspace)? != expected {
        return Ok(false);
    }
    let consumed = source.len().checked_sub(trial.len()).ok_or_else(|| {
        crate::utils::cold_path();
        Error::Internal
    })?;
    advance_input(input, consumed)?;
    Ok(true)
}

#[cfg(test)]
mod ber_tag_boundary_tests {
    use super::*;

    #[test]
    fn malformed_configured_tags_are_debug_errors() {
        for tag in ["9F", "5A5B", "00", "9f02"] {
            let encoded =
                std::panic::catch_unwind(|| encode_ber_tlv_field(&mut &mut [0u8; 16][..], &mut [0u8; 16][..], tag, "field", |_, _| Ok(())));
            assert_eq!(encoded.is_err(), cfg!(debug_assertions));
            // Decoding compares literals as written; a malformed one is caught in debug builds.
            let result = std::panic::catch_unwind(|| {
                decode_ber_tlv_field(
                    b"5A",
                    tag,
                    &mut &[][..],
                    &mut &mut [0u8; 16][..],
                    &mut None::<()>,
                    "field",
                    |_, _| Ok(()),
                )
            });
            assert_eq!(result.is_err(), cfg!(debug_assertions));
            if let Ok(matched) = result {
                assert_eq!(matched, Ok(false));
            }
        }
    }
}

#[cfg(test)]
mod delimited_proptests {
    use proptest::prelude::*;

    use super::*;
    use crate::{Ascii, Field, Rest};

    type Text = Field<Ascii<0, 64>, Rest>;
    #[derive(Debug, PartialEq)]
    struct Record {
        first: String,
        tail: Option<String>,
    }
    crate::delimited_format! { struct Format for Record, b'|' {
        first: Text, tail: Option<Text>,
    } }
    fn roundtrip(first: String, tail: Option<String>) {
        let mut value = Record { first, tail };
        let mut output = [0; 130];
        let used = {
            let mut out = output.as_mut_slice();
            Format::encode_field(&mut out, &mut [], &value).unwrap();
            130 - out.len()
        };
        let mut input = &output[..used];
        value.tail = value.tail.filter(|text| !text.is_empty());
        assert_eq!(Format::decode_field(&mut input, &mut &mut [][..]).unwrap(), value);
        assert!(input.is_empty());
    }
    #[test]
    fn final_segment_separators_and_absence() {
        for tail in [None, Some(""), Some("|"), Some("B|C"), Some("B|")] {
            roundtrip("A".into(), tail.map(str::to_owned));
        }
        let mut output = [0; 16];
        let separator_inside = std::panic::catch_unwind(|| {
            let value = Record {
                first: "A|B".into(),
                tail: None,
            };
            Format::encode_field(&mut &mut [0; 16][..], &mut [], &value)
        });
        assert_eq!(separator_inside.is_err(), cfg!(debug_assertions));
        #[derive(Debug, PartialEq)]
        struct Literal {
            first: String,
        }
        crate::delimited_format! { struct LiteralFormat for Literal, b'|' { first: Text, _: Text = b"B|C", } }
        let value = Literal { first: "A".into() };
        let mut out = output.as_mut_slice();
        LiteralFormat::encode_field(&mut out, &mut [], &value).unwrap();
        let used = 16 - out.len();
        assert_eq!(&output[..used], b"A|B|C");
        assert_eq!(LiteralFormat::decode_field(&mut &output[..used], &mut &mut [][..]).unwrap(), value);
    }
    proptest! {
        #[test]
        fn final_segment_roundtrip(first in "[A-Za-z0-9]{0,32}", tail in prop::option::of("[A-Za-z0-9|]{0,32}")) {
            roundtrip(first, tail);
        }
    }
}

#[cfg(test)]
mod tagged_tests {
    use super::*;
    use crate::{Ascii, Field, Fixed, FixedSignedComp3, Rest};
    type A1 = Field<Ascii<1, 1>, Fixed<1>>;
    type A2 = Field<Ascii<2, 2>, Fixed<2>>;
    #[derive(Debug, PartialEq)]
    enum Value {
        Long(()),
        Short(()),
    }
    crate::tagged_format! { struct Prefix for Value {
        _: A2 = b"AB" => Long(Empty),
        _: A1 = b"A" => Short(Empty),
    } }
    crate::tagged_format! { struct Guard for Value {
        _: A1 = b"A" => Long(Empty) if |remaining| remaining == 1,
        _: A1 = b"A" => Short(Empty),
    } }
    #[test]
    fn partial_tags_and_guards_try_later_arms() {
        for (wire, expected, rest) in [
            (&b"A"[..], Ok(Value::Short(())), &b""[..]),
            (b"AB", Ok(Value::Long(())), b""),
            (b"ABC", Ok(Value::Long(())), b"C"),
            (b"", Err(Error::UnexpectedEof), b""),
            (b"ZZ", Err(Error::Invalid), b"ZZ"),
        ] {
            let mut input = wire;
            assert_eq!(Prefix::decode_field(&mut input, &mut &mut [][..]).map_err(|e| e.kind), expected);
            assert_eq!(input, rest);
        }
        assert_eq!(Guard::decode_field(&mut &b"A"[..], &mut &mut [][..]).unwrap(), Value::Short(()));
        assert_eq!(Guard::decode_field(&mut &b"AB"[..], &mut &mut [][..]).unwrap(), Value::Long(()));
    }
    struct Reject<const CODE: u8>;
    impl<const CODE: u8> ScalarFmt for Reject<CODE> {
        fn encoded_len(value: &[u8]) -> Result<usize, Error> {
            A1::encoded_len(value)
        }
        fn encode(output: &mut &mut [u8], scratch: &mut [u8], value: &[u8]) -> Result<(), Error> {
            A1::encode(output, scratch, value)
        }
        fn decode<'a>(_input: &mut &'a [u8], _scratch: &mut &'a mut [u8]) -> Result<&'a [u8], Error> {
            Err(match CODE {
                0 => Error::Invalid,
                1 => Error::UnexpectedEof,
                2 => Error::BufferOverflow,
                _ => Error::Internal,
            })
        }
    }
    #[test]
    fn tag_errors_retry_but_resource_and_selected_body_errors_do_not() {
        macro_rules! check {
            ($code:literal, $expected:expr) => {{
                crate::tagged_format! { struct Format for Value {
                    _: Reject<$code> = b"A" => Long(Empty),
                    _: A1 = b"A" => Short(Empty),
                } }
                let mut input = &b"A"[..];
                assert_eq!(
                    Format::decode_field(&mut input, &mut &mut [][..]).map_err(|e| e.kind),
                    $expected
                );
                assert_eq!(input, if $code < 2 { &b""[..] } else { &b"A"[..] });
            }};
        }
        check!(0, Ok(Value::Short(())));
        check!(1, Ok(Value::Short(())));
        check!(2, Err(Error::BufferOverflow));
        check!(3, Err(Error::Internal));
        #[derive(Debug, PartialEq)]
        enum Body {
            Required(String),
            Empty(()),
        }
        crate::tagged_format! { struct Format for Body {
            _: A1 = b"A" => Required(A2),
            _: A1 = b"A" => Empty(Empty),
        } }
        assert_eq!(
            Format::decode_field(&mut &b"A"[..], &mut &mut [][..]).unwrap_err().kind,
            Error::UnexpectedEof
        );
    }
    #[test]
    fn literal_semantics_and_temporary_scratch() {
        let mut scratch = [0; 80];
        for wire in [&b"\x1C!"[..], &b"\x1F!"[..]] {
            let mut input = wire;
            let mut workspace = scratch.as_mut_slice();
            assert_eq!(match_literal::<FixedSignedComp3<1>>(&mut input, &mut workspace, b"1"), Ok(true));
            assert_eq!(input, b"!");
            assert_eq!(workspace.len(), 80);
        }
        for expected in [&b"1"[..], &b"01"[..], &b"2"[..]] {
            let mut input = &b"\0\x1C"[..];
            assert_eq!(
                match_literal::<FixedSignedComp3<2>>(&mut input, &mut scratch.as_mut_slice(), expected),
                Ok(expected == b"1")
            );
            assert_eq!(input.len(), if expected == b"1" { 0 } else { 2 });
        }
        type Ebcdic = Field<Ascii<1, 1>, Fixed<1>, crate::Ebcdic037>;
        for expected in [b"A", b"B"] {
            let mut input = &b"\xC1!"[..];
            let mut workspace = &mut scratch[..1];
            assert_eq!(match_literal::<Ebcdic>(&mut input, &mut workspace, expected), Ok(expected == b"A"));
            assert_eq!(workspace.len(), 1);
            assert_eq!(input.len(), if expected == b"A" { 1 } else { 2 });
        }
        type Long = Field<Ascii<0, 80>, Rest>;
        assert_eq!(match_literal::<Long>(&mut &[b'A'; 80][..], &mut &mut [][..], &[b'A'; 80]), Ok(true));
    }
}
