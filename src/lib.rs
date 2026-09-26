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

pub use bitmap::{Bitmap, BitmapLayout, decode_bitmap, encode_bitmap};
pub use composite::{
    AbsentFmt, BerTlvExtras, BerTlvList, BoundedList, ByteFill, Composite, CompositeFmt, ContextFmt, DirectScalar, Empty, FixedAreaList,
    FixedCount, FixedCountList, Frame, NoTrailingFields, OptionalAbsent, ScalarValue, Separator, SerdeScalar, TrailingField,
    TrailingLengthFrame, decode, encode,
};
pub use field::{
    Alpha, Alphanum, Ascii, AsciiLength, AsciiPrintable, AsciiWireLength, Bcd, BcdBytes, Bcdz, Binary, BlankableEbcdicLength, Check,
    DecodeCheck, DecodePlan, Ebcdic037, Ebcdic037Ascii, Ebcdic1142, Ebcdic1142Text, EbcdicLength, EbcdicPrintable, EbcdicWireLength, Field,
    Fixed, FixedBinaryBe, FixedComp3, FixedNibbleInt, FixedSignedBinaryBe, FixedSignedComp3, FixedSignedZonedEbcdic, Hex, HexEven,
    Identity, ImpliedDecimal, Iso88591, Length, LengthSpec, LowerHex, LowerHexEven, MinusPrefix, Numeric, Offset, PackNibbles,
    PackNibblesLeft, PackNibblesRight, PadLeft, PadLeftEven, PadRight, PadRightEven, PaddedField, Rest, SignPrefix, Step, Track2, Truncate,
    UnpackNibbles, UpperAlpha, UpperAlphanum, UpperAsciiPrintable, UpperHex, UpperHexEven, WireLength,
};
pub use scalarfmt::ScalarFmt;
pub use types::{CompositeError, Error, PathSegment};

#[doc(hidden)]
pub mod __private {
    use crate::CompositeError;

    #[inline(always)]
    pub fn cold_path() {
        crate::utils::cold_path();
    }

    #[inline(always)]
    pub fn decode_variant<'a, T, E, F, W>(input: &mut &'a [u8], scratch: &mut &'a mut [u8], wrap: W) -> Result<E, CompositeError>
    where
        F: crate::composite::CompositeFmt<T>,
        W: FnOnce(<F as crate::composite::CompositeFmt<T>>::Decoded<'a>) -> E,
    {
        crate::composite::decode_variant::<T, E, F, W>(input, scratch, wrap)
    }
}
