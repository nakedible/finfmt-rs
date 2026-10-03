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

pub use bitmap::{Bitmap, BitmapFormat, BitmapLayout, decode_bitmap, encode_bitmap};
pub use composite::{
    AbsentBytes, AbsentFmt, BerTlvExtras, BoundedList, ContextDecode, ContextEncode, Empty, FieldDecode, FieldEncode, FixedAreaList, Frame,
    NoTrailingFields, OptionAs, ScalarDecode, ScalarEncode, Separator, TrailingField, TrailingLengthFrame, decode, encode,
};
#[cfg(feature = "serde")]
pub use composite::{BerTlvList, SerdeScalar};
pub use field::{
    Alpha, Alphanum, Ascii, AsciiLength, AsciiPrintable, Bcd, BcdBytes, Bcdz, Binary, BlankableEbcdicLength, Check, ConstBytes, Count,
    DecodeCheck, Ebcdic037, Ebcdic037Ascii, Ebcdic1142, Ebcdic1142Text, EbcdicLength, EbcdicPrintable, Field, Fill, Fixed, FixedBinaryBe,
    FixedComp3, FixedNibbleInt, FixedSignedBinaryBe, FixedSignedComp3, FixedSignedZonedEbcdic, Hex, HexEven, Identity, ImpliedDecimal,
    Iso88591, Length, LengthSpec, LowerHex, LowerHexEven, MinusPrefix, Numeric, Offset, PackNibbles, PackNibblesLeft, PackNibblesRight,
    PadLeft, PadLeftEven, PadRight, PadRightEven, Per, Rest, SignPrefix, Step, Track2, Truncate, UnpackNibbles, UpperAlpha, UpperAlphanum,
    UpperAsciiPrintable, UpperHex, UpperHexEven,
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
    pub fn encode_variant<T: ?Sized, F: crate::composite::FieldEncode<T>>(
        output: &mut &mut [u8],
        scratch: &mut [u8],
        value: &T,
    ) -> Result<(), CompositeError> {
        crate::composite::encode_variant::<T, F>(output, scratch, value)
    }

    #[inline(always)]
    pub fn decode_variant<'a, T, E, F, W>(input: &mut &'a [u8], scratch: &mut &'a mut [u8], wrap: W) -> Result<E, CompositeError>
    where
        F: crate::composite::FieldDecode<'a, T>,
        W: FnOnce(T) -> E,
    {
        crate::composite::decode_variant::<T, E, F, W>(input, scratch, wrap)
    }
}
