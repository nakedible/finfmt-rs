//! Composable field operations.

mod bertlv;
mod check;
mod codepage;
mod constant;
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

pub use bertlv::{BerLength, BerTag, StrictBerTag};
pub use check::{
    Alpha, Alphanum, Ascii, AsciiPrintable, AsciiSubsetBytes, Bcd, BcdBytes, Bcdz, Binary, CharsetText, Check, EbcdicPrintable, Hex,
    HexEven, LowerHex, LowerHexEven, Numeric, Track2, UpperAlpha, UpperAlphanum, UpperAsciiPrintable, UpperHex, UpperHexEven,
};
pub use codepage::{AsciiSubset, Charset};
pub use constant::{ConstBytes, Fill};
pub use format::Field;
pub(crate) use format::{decode_framed, encode_length, encode_steps};
pub use length::{AsciiLength, BlankableEbcdicLength, EbcdicLength, Fixed, Length, LengthSpec, Offset, Per, Rest};
pub use nibble::{PackNibbles, PackNibblesLeft, PackNibblesRight, UnpackNibbles};
pub use numeric::{
    FixedBinaryBe, FixedComp3, FixedNibbleInt, FixedSignedBinaryBe, FixedSignedComp3, FixedSignedZonedAscii, FixedSignedZonedEbcdic,
    ImpliedDecimal, MinusPrefix, SignPrefix, SignSuffix,
};
pub use step::{Chain, Count, DecodeCheck, Step};
pub use text::{Identity, PadLeft, PadLeftEven, PadRight, PadRightEven};
pub use truncate::Truncate;
