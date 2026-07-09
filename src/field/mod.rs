//! Composable field operations.

mod check;
mod ebcdic;
mod format;
mod length;
mod nibble;
mod numeric;
mod step;
mod text;

#[macro_export]
macro_rules! chain {
    ($step:ty $(,)?) => {
        $step
    };
    ($first:ty, $($rest:ty),+ $(,)?) => {
        $crate::field::Chain<$first, $crate::chain!($($rest),+)>
    };
}

pub use check::{
    Alpha, Alphanum, Ascii, AsciiPrintable, Bcd, BcdBytes, Bcdz, Binary, Check, Ebcdic1142Text, EbcdicPrintable, Hex, HexEven, HexLower,
    HexLowerEven, HexUpper, HexUpperEven, Iso88591, Numeric, Track2, UpperAlpha, UpperAlphanum, UpperAsciiPrintable,
};
pub use ebcdic::{Ebcdic037, Ebcdic1142};
pub use format::{Field, PaddedField};
pub use length::{
    AsciiLength, AsciiWireLength, BlankableEbcdicLength, DecodePlan, EbcdicLength, EbcdicWireLength, Fixed, Length, LengthSpec, Rest,
    WireFixed, WireLength,
};
pub use nibble::{PackNibbles, PackNibblesLeft, PackNibblesRight, UnpackNibbles};
pub use numeric::{
    FixedBinaryBe, FixedComp3, FixedNibbleInt, FixedSignedBinaryBe, FixedSignedComp3, FixedSignedZonedEbcdic, ImpliedDecimal, MinusPrefix,
    SignPrefix,
};
pub use step::{ByteCheck, Chain, Step};
pub use text::{Identity, PadLeft, PadLeftEven, PadRight, PadRightEven};
