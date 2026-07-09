use core::marker::PhantomData;

use serde::de::value::StrDeserializer;
use serde::de::{self, DeserializeOwned, Visitor};
use serde::ser::{self, Impossible};
use serde::{Deserialize, Serialize};

use super::*;

/// Wrap a leaf [`ScalarFmt`] as a [`CompositeFmt`] through Serde scalar values.
///
/// This is the Serde-backed scalar adapter used by the struct-format macros.
/// It is also useful when a custom composite wrapper needs to reuse a scalar
/// field format for a type that already implements Serde but does not implement
/// [`ScalarValue`].
pub struct SerdeScalar<F>(PhantomData<F>);

impl ser::Error for Error {
    #[inline(always)]
    fn custom<T: core::fmt::Display>(_msg: T) -> Self {
        crate::utils::cold_path();
        Error::Invalid
    }
}

impl de::Error for Error {
    #[inline(always)]
    fn custom<T: core::fmt::Display>(_msg: T) -> Self {
        crate::utils::cold_path();
        Error::Invalid
    }
}

struct ScalarValueSerializer<'a, 'out, 'scratch, F: ScalarFmt> {
    output: &'a mut &'out mut [u8],
    scratch: &'a mut &'scratch mut [u8],
    _marker: PhantomData<F>,
}

impl<F: ScalarFmt> ScalarValueSerializer<'_, '_, '_, F> {
    #[inline(always)]
    fn encode_str(self, value: &str) -> Result<(), Error> {
        F::encode_str(self.output, self.scratch, value)
    }

    #[inline(always)]
    fn encode_u64(self, value: u64) -> Result<(), Error> {
        F::encode_u64(self.output, self.scratch, value)
    }

    #[inline(always)]
    fn encode_i64(self, value: i64) -> Result<(), Error> {
        F::encode_i64(self.output, self.scratch, value)
    }
}

impl<F: ScalarFmt> serde::Serializer for ScalarValueSerializer<'_, '_, '_, F> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Impossible<(), Error>;
    type SerializeTuple = Impossible<(), Error>;
    type SerializeTupleStruct = Impossible<(), Error>;
    type SerializeTupleVariant = Impossible<(), Error>;
    type SerializeMap = Impossible<(), Error>;
    type SerializeStruct = Impossible<(), Error>;
    type SerializeStructVariant = Impossible<(), Error>;

    #[inline(always)]
    fn serialize_bool(self, _v: bool) -> Result<Self::Ok, Self::Error> {
        crate::utils::cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.encode_i64(v as i64)
    }

    #[inline(always)]
    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.encode_i64(v as i64)
    }

    #[inline(always)]
    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.encode_i64(v as i64)
    }

    #[inline(always)]
    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        self.encode_i64(v)
    }

    #[inline(always)]
    fn serialize_i128(self, v: i128) -> Result<Self::Ok, Self::Error> {
        self.encode_i64(i64::try_from(v).map_err(|_| {
            crate::utils::cold_path();
            Error::Invalid
        })?)
    }

    #[inline(always)]
    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.encode_u64(v as u64)
    }

    #[inline(always)]
    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.encode_u64(v as u64)
    }

    #[inline(always)]
    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.encode_u64(v as u64)
    }

    #[inline(always)]
    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        self.encode_u64(v)
    }

    #[inline(always)]
    fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> {
        self.encode_u64(u64::try_from(v).map_err(|_| {
            crate::utils::cold_path();
            Error::Invalid
        })?)
    }

    #[inline(always)]
    fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
        crate::utils::cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> {
        crate::utils::cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        let mut buf = [0u8; 4];
        self.encode_str(v.encode_utf8(&mut buf))
    }

    #[inline(always)]
    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.encode_str(v)
    }

    #[inline(always)]
    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
        crate::utils::cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        crate::utils::cold_path();
        Err(Error::Invalid)
    }

    #[inline(always)]
    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    #[inline(always)]
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        crate::utils::cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        crate::utils::cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_unit_variant(self, _name: &'static str, _variant_index: u32, variant: &'static str) -> Result<Self::Ok, Self::Error> {
        self.encode_str(variant)
    }

    #[inline(always)]
    fn serialize_newtype_struct<T: ?Sized + Serialize>(self, _name: &'static str, value: &T) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    #[inline(always)]
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        crate::utils::cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        crate::utils::cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        crate::utils::cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_tuple_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeTupleStruct, Self::Error> {
        crate::utils::cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        crate::utils::cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        crate::utils::cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct, Self::Error> {
        crate::utils::cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        crate::utils::cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn is_human_readable(&self) -> bool {
        true
    }
}

struct ScalarValueDeserializer<'a, 'b, F: ScalarFmt> {
    input: &'a mut &'b [u8],
    scratch: &'a mut &'b mut [u8],
    _marker: PhantomData<F>,
}

impl<'a, 'de, F: ScalarFmt> ScalarValueDeserializer<'a, 'de, F> {
    #[inline(always)]
    fn decode_u64(&mut self) -> Result<u64, Error> {
        F::decode_u64(self.input, self.scratch)
    }

    #[inline(always)]
    fn decode_i64(&mut self) -> Result<i64, Error> {
        F::decode_i64(self.input, self.scratch)
    }

    #[inline(always)]
    fn decode_str(&mut self) -> Result<&'de str, Error> {
        F::decode_str(self.input, self.scratch)
    }
}

impl<'de, F: ScalarFmt> serde::Deserializer<'de> for ScalarValueDeserializer<'_, 'de, F> {
    type Error = Error;

    #[inline(always)]
    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        crate::utils::cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn deserialize_bool<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        crate::utils::cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn deserialize_i8<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = i8::try_from(self.decode_i64()?).map_err(|_| {
            crate::utils::cold_path();
            Error::Invalid
        })?;
        visitor.visit_i8(value)
    }

    #[inline(always)]
    fn deserialize_i16<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = i16::try_from(self.decode_i64()?).map_err(|_| {
            crate::utils::cold_path();
            Error::Invalid
        })?;
        visitor.visit_i16(value)
    }

    #[inline(always)]
    fn deserialize_i32<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = i32::try_from(self.decode_i64()?).map_err(|_| {
            crate::utils::cold_path();
            Error::Invalid
        })?;
        visitor.visit_i32(value)
    }

    #[inline(always)]
    fn deserialize_i64<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i64(self.decode_i64()?)
    }

    #[inline(always)]
    fn deserialize_i128<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i128(i128::from(self.decode_i64()?))
    }

    #[inline(always)]
    fn deserialize_u8<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = u8::try_from(self.decode_u64()?).map_err(|_| {
            crate::utils::cold_path();
            Error::Invalid
        })?;
        visitor.visit_u8(value)
    }

    #[inline(always)]
    fn deserialize_u16<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = u16::try_from(self.decode_u64()?).map_err(|_| {
            crate::utils::cold_path();
            Error::Invalid
        })?;
        visitor.visit_u16(value)
    }

    #[inline(always)]
    fn deserialize_u32<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = u32::try_from(self.decode_u64()?).map_err(|_| {
            crate::utils::cold_path();
            Error::Invalid
        })?;
        visitor.visit_u32(value)
    }

    #[inline(always)]
    fn deserialize_u64<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(self.decode_u64()?)
    }

    #[inline(always)]
    fn deserialize_u128<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u128(u128::from(self.decode_u64()?))
    }

    #[inline(always)]
    fn deserialize_f32<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        crate::utils::cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        crate::utils::cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn deserialize_char<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = self.decode_str().and_then(|s| {
            let mut chars = s.chars();
            match (chars.next(), chars.next()) {
                (Some(ch), None) => Ok(ch),
                _ => {
                    crate::utils::cold_path();
                    Err(Error::Invalid)
                }
            }
        })?;
        visitor.visit_char(value)
    }

    #[inline(always)]
    fn deserialize_str<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_str(self.decode_str()?)
    }

    #[inline(always)]
    fn deserialize_string<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_str(self.decode_str()?)
    }

    #[inline(always)]
    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        crate::utils::cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        crate::utils::cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    #[inline(always)]
    fn deserialize_unit<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        crate::utils::cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn deserialize_unit_struct<V>(self, _name: &'static str, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        crate::utils::cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    #[inline(always)]
    fn deserialize_seq<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        crate::utils::cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn deserialize_tuple<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        crate::utils::cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn deserialize_tuple_struct<V>(self, _name: &'static str, _len: usize, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        crate::utils::cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn deserialize_map<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        crate::utils::cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn deserialize_struct<V>(self, _name: &'static str, _fields: &'static [&'static str], _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        crate::utils::cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn deserialize_enum<V>(mut self, _name: &'static str, _variants: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_enum(StrDeserializer::<Error>::new(self.decode_str()?))
    }

    #[inline(always)]
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    #[inline(always)]
    fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        crate::utils::cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn is_human_readable(&self) -> bool {
        true
    }
}

#[inline(always)]
pub fn encode_serde_scalar<T, F: ScalarFmt>(value: &T, output: &mut &mut [u8], scratch: &mut &mut [u8]) -> Result<(), Error>
where
    T: ?Sized + Serialize,
{
    value.serialize(ScalarValueSerializer::<F> {
        output,
        scratch,
        _marker: PhantomData,
    })
}

#[inline(always)]
pub fn decode_serde_scalar<'a, T, F: ScalarFmt>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<T, Error>
where
    T: Deserialize<'a>,
{
    T::deserialize(ScalarValueDeserializer::<F> {
        input,
        scratch,
        _marker: PhantomData,
    })
}

impl<T, F: ScalarFmt> CompositeFmt<T> for SerdeScalar<F>
where
    T: Serialize + DeserializeOwned,
{
    type Decoded<'de> = T;

    #[inline(always)]
    fn encode_cursor(output: &mut &mut [u8], scratch: &mut &mut [u8], value: &T) -> Result<(), StructError> {
        encode_serde_scalar::<T, F>(value, output, scratch)?;
        Ok(())
    }

    #[inline(always)]
    fn decode_cursor<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<T, StructError> {
        let value = decode_serde_scalar::<T, F>(input, scratch)?;
        Ok(value)
    }
}
