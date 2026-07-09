use core::marker::PhantomData;

use compact_str::CompactString;

use super::*;

/// Wrap a leaf [`ScalarFmt`] as a [`CompositeFmt`] using [`ScalarValue`].
///
/// This is the direct scalar escape hatch for composite formats: implement
/// [`ScalarValue`] for a custom scalar type and then use
/// `field: DirectScalar<Fmt>` in the composite format definition.
///
/// Use `DirectScalar<Fmt, &str>` for borrowed string decode.
pub struct DirectScalar<F, T = ()>(PhantomData<(F, T)>);

/// Mark a field declaration as a nested [`CompositeFmt`].
///
/// This is primarily used by the format macros through `field: Composite<Fmt>`,
/// but is also a zero-cost adapter for APIs that expect a format type.
pub struct Composite<F>(PhantomData<F>);

/// Scalar value adapter used by [`DirectScalar`].
///
/// The normal scalar field syntax in composite macros (`field: Fmt`) uses the
/// crate's default Serde-based scalar adapter. User-defined scalar types that
/// need custom encode/decode behavior can implement this trait directly and
/// then use `field: DirectScalar<Fmt>` in the composite format definition.
pub trait ScalarValue {
    fn encode_with<F: ScalarFmt>(&self, output: &mut &mut [u8], scratch: &mut &mut [u8]) -> Result<(), Error>;
    fn decode_with<'de, F: ScalarFmt>(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self, Error>
    where
        Self: Sized;
}

impl ScalarValue for String {
    #[inline(always)]
    fn encode_with<F: ScalarFmt>(&self, output: &mut &mut [u8], scratch: &mut &mut [u8]) -> Result<(), Error> {
        F::encode_str(output, scratch, self)
    }

    #[inline(always)]
    fn decode_with<'de, F: ScalarFmt>(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self, Error> {
        Ok(F::decode_str(input, scratch)?.to_owned())
    }
}

impl ScalarValue for CompactString {
    #[inline(always)]
    fn encode_with<F: ScalarFmt>(&self, output: &mut &mut [u8], scratch: &mut &mut [u8]) -> Result<(), Error> {
        F::encode_str(output, scratch, self.as_str())
    }

    #[inline(always)]
    fn decode_with<'de, F: ScalarFmt>(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self, Error> {
        Ok(CompactString::from(F::decode_str(input, scratch)?))
    }
}

impl ScalarValue for u64 {
    #[inline(always)]
    fn encode_with<F: ScalarFmt>(&self, output: &mut &mut [u8], scratch: &mut &mut [u8]) -> Result<(), Error> {
        F::encode_u64(output, scratch, *self)
    }

    #[inline(always)]
    fn decode_with<'de, F: ScalarFmt>(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self, Error> {
        F::decode_u64(input, scratch)
    }
}

impl ScalarValue for i64 {
    #[inline(always)]
    fn encode_with<F: ScalarFmt>(&self, output: &mut &mut [u8], scratch: &mut &mut [u8]) -> Result<(), Error> {
        F::encode_i64(output, scratch, *self)
    }

    #[inline(always)]
    fn decode_with<'de, F: ScalarFmt>(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self, Error> {
        F::decode_i64(input, scratch)
    }
}

impl<T: ScalarValue, F: ScalarFmt> CompositeFmt<T> for DirectScalar<F> {
    type Decoded<'de> = T;

    #[inline(always)]
    fn encode_cursor(output: &mut &mut [u8], scratch: &mut &mut [u8], value: &T) -> Result<(), StructError> {
        T::encode_with::<F>(value, output, scratch)?;
        Ok(())
    }

    #[inline(always)]
    fn decode_cursor<'de>(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<T, StructError> {
        let value = T::decode_with::<F>(input, scratch)?;
        Ok(value)
    }
}

impl<'value, F: ScalarFmt> CompositeFmt<&'value str> for DirectScalar<F, &'value str> {
    type Decoded<'de> = &'de str;

    #[inline(always)]
    fn encode_cursor(output: &mut &mut [u8], scratch: &mut &mut [u8], value: &&'value str) -> Result<(), StructError> {
        F::encode_str(output, scratch, value)?;
        Ok(())
    }

    #[inline(always)]
    fn decode_cursor<'de>(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self::Decoded<'de>, StructError> {
        Ok(F::decode_str(input, scratch)?)
    }
}

impl<T, F: CompositeFmt<T>> CompositeFmt<T> for Composite<F> {
    type Decoded<'de> = F::Decoded<'de>;

    #[inline(always)]
    fn encode_cursor(output: &mut &mut [u8], scratch: &mut &mut [u8], value: &T) -> Result<(), StructError> {
        F::encode_cursor(output, scratch, value)
    }

    #[inline(always)]
    fn decode_cursor<'de>(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self::Decoded<'de>, StructError> {
        F::decode_cursor(input, scratch)
    }
}
