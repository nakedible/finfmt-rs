//! Composable field operations.

mod check;
mod constant;
mod ebcdic;
mod format;
mod length;
mod nibble;
mod numeric;
mod step;
mod text;
mod truncate;

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
    Alpha, Alphanum, Ascii, AsciiPrintable, Bcd, BcdBytes, Bcdz, Binary, Check, Ebcdic037Ascii, Ebcdic1142Text, EbcdicPrintable, Hex,
    HexEven, Iso88591, LowerHex, LowerHexEven, Numeric, Track2, UpperAlpha, UpperAlphanum, UpperAsciiPrintable, UpperHex, UpperHexEven,
};
pub use constant::{ConstBytes, Fill};
pub use ebcdic::{Ebcdic037, Ebcdic1142};
pub use format::Field;
pub use length::{AsciiLength, BlankableEbcdicLength, EbcdicLength, Fixed, Length, LengthSpec, Offset, Per, Rest};
pub use nibble::{PackNibbles, PackNibblesLeft, PackNibblesRight, UnpackNibbles};
pub use numeric::{
    FixedBinaryBe, FixedComp3, FixedNibbleInt, FixedSignedBinaryBe, FixedSignedComp3, FixedSignedZonedEbcdic, ImpliedDecimal, MinusPrefix,
    SignPrefix,
};
pub use step::{Chain, Count, DecodeCheck, Step};
pub use text::{Identity, PadLeft, PadLeftEven, PadRight, PadRightEven};
pub use truncate::Truncate;
