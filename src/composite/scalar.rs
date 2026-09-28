use super::*;

/// A Rust value that scalar formats can encode: text goes through
/// [`ScalarFmt::encode_str`], integers through the typed numeric methods.
///
/// A type without a mapping is not a field value:
///
/// ```compile_fail
/// use finfmt::{Ascii, Field, FieldEncode, Fixed};
/// let _ = <Field<Ascii<1, 1>, Fixed<1>> as FieldEncode<bool>>::encode_field(&mut &mut [][..], &mut [], &true);
/// ```
///
/// The crate implements it for `str`, `String`, `Box<str>` and `u64`, `i64`,
/// `usize`, and for `CompactString` with the `compact_str` feature. Implement
/// it for your own types to give them a wire mapping independent of their
/// serde representation. A value whose serde representation should be used on
/// the wire goes through [`SerdeScalar`] instead.
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no built-in scalar mapping",
    label = "no `ScalarEncode` implementation",
    note = "implement `ScalarEncode` for your own type, or use `SerdeScalar<Fmt>` to encode it through serde"
)]
pub trait ScalarEncode {
    fn encode_scalar<F: ScalarFmt>(&self, output: &mut &mut [u8], scratch: &mut [u8]) -> Result<(), Error>;
}

/// A Rust value that scalar formats can decode, possibly borrowing input or
/// scratch for `'de`, as `&'de str` does. See [`ScalarEncode`].
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no built-in scalar mapping",
    label = "no `ScalarDecode` implementation",
    note = "implement `ScalarDecode` for your own type, or use `SerdeScalar<Fmt>` to decode it through serde"
)]
pub trait ScalarDecode<'de>: Sized {
    fn decode_scalar<F: ScalarFmt>(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self, Error>;
}

impl<T: ?Sized + ScalarEncode, F: ScalarFmt> FieldEncode<T> for F {
    #[inline]
    fn encode_field(output: &mut &mut [u8], scratch: &mut [u8], value: &T) -> Result<(), CompositeError> {
        value.encode_scalar::<F>(output, scratch)?;
        Ok(())
    }
}

impl<'de, T: ScalarDecode<'de>, F: ScalarFmt> FieldDecode<'de, T> for F {
    const TAKES_REST: bool = F::TAKES_REST;

    #[inline]
    fn decode_field(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<T, CompositeError> {
        Ok(T::decode_scalar::<F>(input, scratch)?)
    }
}

impl<T: ?Sized + ScalarEncode> ScalarEncode for &T {
    #[inline]
    fn encode_scalar<F: ScalarFmt>(&self, output: &mut &mut [u8], scratch: &mut [u8]) -> Result<(), Error> {
        (**self).encode_scalar::<F>(output, scratch)
    }
}

impl ScalarEncode for str {
    #[inline]
    fn encode_scalar<F: ScalarFmt>(&self, output: &mut &mut [u8], scratch: &mut [u8]) -> Result<(), Error> {
        F::encode_str(output, scratch, self)
    }
}

impl<'de> ScalarDecode<'de> for &'de str {
    #[inline]
    fn decode_scalar<F: ScalarFmt>(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self, Error> {
        F::decode_str(input, scratch)
    }
}

macro_rules! owned_text_value {
    ($($ty:ty => $from:expr),* $(,)?) => {$(
        impl ScalarEncode for $ty {
            #[inline]
            fn encode_scalar<F: ScalarFmt>(&self, output: &mut &mut [u8], scratch: &mut [u8]) -> Result<(), Error> {
                F::encode_str(output, scratch, self)
            }
        }

        impl<'de> ScalarDecode<'de> for $ty {
            #[inline]
            fn decode_scalar<F: ScalarFmt>(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self, Error> {
                F::decode_str(input, scratch).map($from)
            }
        }
    )*};
}

owned_text_value! {
    String => String::from,
    Box<str> => Box::from,
}

#[cfg(feature = "compact_str")]
owned_text_value! {
    compact_str::CompactString => compact_str::CompactString::from,
}

macro_rules! integer_value {
    ($($ty:ty => $encode:ident, $decode:ident);* $(;)?) => {$(
        impl ScalarEncode for $ty {
            #[inline]
            fn encode_scalar<F: ScalarFmt>(&self, output: &mut &mut [u8], scratch: &mut [u8]) -> Result<(), Error> {
                F::$encode(output, scratch, *self)
            }
        }

        impl<'de> ScalarDecode<'de> for $ty {
            #[inline]
            fn decode_scalar<F: ScalarFmt>(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self, Error> {
                F::$decode(input, scratch)
            }
        }
    )*};
}

integer_value! {
    u64 => encode_u64, decode_u64;
    i64 => encode_i64, decode_i64;
    usize => encode_usize, decode_usize;
}

#[cfg(test)]
mod tests {
    use crate::{Ascii, AsciiLength, Field, FieldDecode, FieldEncode, FixedBinaryBe, Numeric, SignPrefix, decode, encode};

    type Text = Field<Ascii<0, 9>, AsciiLength<1>>;
    type Number = Field<Numeric<1, 9>, AsciiLength<1>>;

    fn roundtrip<F, T>(value: &T, wire: &[u8]) -> T
    where
        F: FieldEncode<T> + for<'de> FieldDecode<'de, T>,
    {
        let mut output = [0; 16];
        let used = encode::<F, T>(&mut output, &mut [0; 16], value).unwrap();
        assert_eq!(&output[..used], wire);
        decode::<F, T>(&output[..used], &mut [0; 16]).unwrap()
    }

    #[test]
    fn built_in_values_map_to_text_or_numbers() {
        assert_eq!(roundtrip::<Text, String>(&"ABC".into(), b"3ABC"), "ABC");
        assert_eq!(&*roundtrip::<Text, Box<str>>(&"ABC".into(), b"3ABC"), "ABC");
        assert_eq!(roundtrip::<Number, u64>(&42, b"242"), 42);
        assert_eq!(roundtrip::<Number, usize>(&42, b"242"), 42);
        assert_eq!(roundtrip::<SignPrefix<Number>, i64>(&-42, b"D242"), -42);
        // A binary integer field takes the number directly, with no text in between.
        assert_eq!(roundtrip::<FixedBinaryBe<2>, u64>(&0x1234, b"\x12\x34"), 0x1234);
        // References encode like the value they point to.
        let mut output = [0; 8];
        let used = encode::<Text, &String>(&mut output, &mut [], &&String::from("AB")).unwrap();
        assert_eq!(&output[..used], b"2AB");
        let used = encode::<Text, str>(&mut output, &mut [], "AB").unwrap();
        assert_eq!(&output[..used], b"2AB");
    }

    #[test]
    fn borrowed_text_decodes_without_copying() {
        let wire = b"3ABC";
        let text: &str = decode::<Text, &str>(wire, &mut []).unwrap();
        assert_eq!(text, "ABC");
        assert_eq!(text.as_ptr(), wire[1..].as_ptr());
    }

    #[test]
    #[cfg(feature = "compact_str")]
    fn compact_strings_are_text_values() {
        use compact_str::CompactString;
        assert_eq!(roundtrip::<Text, CompactString>(&"ABC".into(), b"3ABC"), "ABC");
    }
}
