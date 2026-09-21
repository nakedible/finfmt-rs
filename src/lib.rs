#![forbid(unsafe_code)]

#[cfg(feature = "asm-inspect")]
pub mod asm;
pub mod bitmap;
pub mod composite;
pub mod field;
pub mod primitive;
mod scalarfmt;
mod types;
mod utils;

pub use bitmap::{Bitmap, BitmapLayout, BitmapWord, decode_bitmap, encode_bitmap};
pub use composite::{
    AbsentFmt, BerTlvExtras, BerTlvList, BoundedList, ByteFill, Composite, CompositeFmt, ContextFmt, DirectScalar, Empty, FixedAreaList,
    FixedCount, FixedCountList, Frame, NoTrailingFields, OptionalAbsent, ScalarValue, Separator, SerdeScalar, TrailingField,
    TrailingLengthFrame,
};
pub use field::{
    Alpha, Alphanum, Ascii, AsciiLength, AsciiPrintable, AsciiWireLength, Bcd, BcdBytes, Bcdz, Binary, BlankableEbcdicLength, Check,
    DecodeCheck, DecodePlan, Ebcdic037, Ebcdic037Ascii, Ebcdic1142, Ebcdic1142Text, EbcdicLength, EbcdicPrintable, EbcdicWireLength, Field,
    Fixed, FixedBinaryBe, FixedComp3, FixedNibbleInt, FixedSignedBinaryBe, FixedSignedComp3, FixedSignedZonedEbcdic, Hex, HexEven,
    HexLower, HexLowerEven, HexUpper, HexUpperEven, Identity, ImpliedDecimal, Iso88591, Length, LengthSpec, MinusPrefix, Numeric,
    PackNibbles, PackNibblesLeft, PackNibblesRight, PadLeft, PadLeftEven, PadRight, PadRightEven, PaddedField, Rest, SignPrefix, Step,
    Track2, TruncateBytes, UnpackNibbles, UpperAlpha, UpperAlphanum, UpperAsciiPrintable, WireFixed, WireLength,
};
pub use scalarfmt::ScalarFmt;
pub use types::{Error, StructError};

#[doc(hidden)]
pub mod __private {
    use crate::{Error, StructError};

    #[inline(always)]
    pub fn cold_path() {
        crate::utils::cold_path();
    }

    #[inline(always)]
    pub fn ber_length_width(len: usize) -> Result<usize, Error> {
        crate::primitive::bertlv::ber_length_width(len)
    }

    #[inline(always)]
    pub fn parse_ber_tag_hex(tag: &str) -> Result<([u8; crate::primitive::bertlv::MAX_BER_TAG_BYTES], usize), Error> {
        crate::primitive::bertlv::parse_ber_tag_hex(tag)
    }

    #[inline(always)]
    pub fn ber_tag_matches_hex(tag_bytes: &[u8], tag_hex: &str) -> Result<bool, Error> {
        crate::primitive::bertlv::ber_tag_matches_hex(tag_bytes, tag_hex)
    }

    #[inline(always)]
    pub fn decode_variant<'a, T, E, F, W>(input: &mut &'a [u8], scratch: &mut &'a mut [u8], wrap: W) -> Result<E, StructError>
    where
        F: crate::composite::CompositeFmt<T>,
        W: FnOnce(<F as crate::composite::CompositeFmt<T>>::Decoded<'a>) -> E,
    {
        crate::composite::decode_variant::<T, E, F, W>(input, scratch, wrap)
    }
}
