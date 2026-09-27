use core::marker::PhantomData;

use serde::de::{self, Visitor};
use serde::ser::{self, Impossible};
use serde::{Deserialize, Serialize};

use super::*;

/// Encode and decode a value through its serde implementation with the
/// scalar format `F`, for types without a [`ScalarEncode`]/[`ScalarDecode`]
/// mapping.
///
/// This opts the field into serde's mapping, so the value's serde attributes
/// and impls decide its wire text. A serde value that is a string or an
/// integer maps onto `F`'s text or numeric methods; borrowed strings decode
/// without copying. Enums are rejected with `Internal`: their serde names
/// describe the JSON form, and their wire mapping is their own
/// `ScalarEncode`/`ScalarDecode` implementation.
///
/// Serializers using `collect_str` format their text into caller-provided scratch
/// before field encoding. Scratch must fit that text plus the field's workspace.
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

struct ScalarValueSerializer<'a, 'out, F: ScalarFmt> {
    output: &'a mut &'out mut [u8],
    scratch: &'a mut [u8],
    _marker: PhantomData<F>,
}

impl<F: ScalarFmt> ScalarValueSerializer<'_, '_, F> {
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

impl<F: ScalarFmt> serde::Serializer for ScalarValueSerializer<'_, '_, F> {
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
    fn serialize_unit_variant(self, _name: &'static str, _variant_index: u32, _variant: &'static str) -> Result<Self::Ok, Self::Error> {
        // An enum's serde names describe its JSON form; its wire mapping is
        // its own ScalarEncode impl.
        crate::utils::cold_path();
        Err(Error::Internal)
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
    fn collect_str<T: ?Sized + core::fmt::Display>(self, value: &T) -> Result<(), Error> {
        use std::io::Write;

        let capacity = self.scratch.len();
        let used = {
            let mut remaining = &mut *self.scratch;
            write!(&mut remaining, "{value}").map_err(|_| {
                crate::utils::cold_path();
                Error::BufferOverflow
            })?;
            capacity - remaining.len()
        };
        let (text, scratch) = split_scratch(self.scratch, used)?;
        let text = core::str::from_utf8(text).map_err(|_| {
            crate::utils::cold_path();
            Error::Internal
        })?;
        F::encode_str(self.output, scratch, text)
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
    fn deserialize_enum<V>(self, _name: &'static str, _variants: &'static [&'static str], _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        crate::utils::cold_path();
        Err(Error::Internal)
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
pub fn encode_serde_scalar<T, F: ScalarFmt>(value: &T, output: &mut &mut [u8], scratch: &mut [u8]) -> Result<(), Error>
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

impl<T: ?Sized + Serialize, F: ScalarFmt> FieldEncode<T> for SerdeScalar<F> {
    #[inline(always)]
    fn encode_field(output: &mut &mut [u8], scratch: &mut [u8], value: &T) -> Result<(), CompositeError> {
        encode_serde_scalar::<T, F>(value, output, scratch)?;
        Ok(())
    }
}

impl<'de, T: Deserialize<'de>, F: ScalarFmt> FieldDecode<'de, T> for SerdeScalar<F> {
    #[inline(always)]
    fn decode_field(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<T, CompositeError> {
        Ok(decode_serde_scalar::<T, F>(input, scratch)?)
    }
}

#[cfg(test)]
mod tests {
    use core::fmt;

    use super::*;
    use crate::{Ascii, Binary, Ebcdic037, Field, Fixed, PadRight, Rest, Truncate};

    type A4 = Field<Ascii<4, 4>, Fixed<4>>;
    type Text = Field<Binary<0, 64>, Rest>;

    struct Parts<'a>(&'a [&'a str]);

    impl fmt::Display for Parts<'_> {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            for part in self.0 {
                formatter.write_str(part)?;
            }
            Ok(())
        }
    }

    fn encode_display<F: ScalarFmt>(value: &impl fmt::Display, output_len: usize, scratch_len: usize) -> Result<Vec<u8>, Error> {
        let mut output = [0xAA; 64];
        let mut scratch = [0xAA; 128];
        let mut out = &mut output[..output_len];
        encode_serde_scalar::<_, F>(&format_args!("{value}"), &mut out, &mut scratch[..scratch_len])?;
        let used = output_len - out.len();
        assert!(out.iter().all(|&byte| byte == 0xAA));
        assert!(scratch[scratch_len..].iter().all(|&byte| byte == 0xAA));
        Ok(output[..used].to_vec())
    }

    #[test]
    fn collect_str_uses_bounded_scratch_and_preserves_field_behavior() {
        let value = Parts(&["A", "B", "CD"]);
        assert_eq!(encode_display::<A4>(&value, 4, 4), Ok(b"ABCD".to_vec()));
        assert_eq!(encode_display::<A4>(&value, 8, 32), Ok(b"ABCD".to_vec()));
        assert_eq!(encode_display::<A4>(&value, 4, 0), Err(Error::BufferOverflow));
        assert_eq!(encode_display::<A4>(&value, 4, 3), Err(Error::BufferOverflow));
        assert_eq!(encode_display::<A4>(&value, 3, 4), Err(Error::BufferOverflow));
        assert_eq!(encode_display::<Text>(&Parts(&[]), 0, 0), Ok(vec![]));
        assert_eq!(encode_display::<Text>(&Parts(&["é", "€"]), 5, 5), Ok("é€".as_bytes().to_vec()));
        assert_eq!(encode_display::<Text>(&Parts(&["é", "€"]), 5, 4), Err(Error::BufferOverflow));
        assert_eq!(encode_display::<A4>(&"ABC", 4, 4), Err(Error::InvalidValueLength));
        assert_eq!(encode_display::<A4>(&"éé", 4, 4), Err(Error::Invalid));
        type E4 = Field<Ascii<4, 4>, Fixed<4>, Ebcdic037>;
        assert_eq!(encode_display::<E4>(&value, 4, 4), Ok(vec![0xC1, 0xC2, 0xC3, 0xC4]));
        type Temp4 = Field<Ascii<4, 4>, Fixed<4>, crate::chain!(PadRight<4>, crate::Ebcdic1142)>;
        assert_eq!(encode_display::<Temp4>(&value, 4, 8), Ok(vec![0xC1, 0xC2, 0xC3, 0xC4]));
        assert_eq!(encode_display::<Temp4>(&value, 4, 7), Err(Error::BufferOverflow));
        type Cut = Truncate<Field<Binary<1, 1>, Fixed<1>>, 1>;
        assert_eq!(encode_display::<Cut>(&"é", 1, 2), Err(Error::InvalidValueLength));
    }

    #[test]
    fn collect_str_formats_once() {
        struct Counted(core::cell::Cell<usize>);
        impl fmt::Display for Counted {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.set(self.0.get() + 1);
                formatter.write_str("ABCD")
            }
        }
        let value = Counted(core::cell::Cell::new(0));
        assert_eq!(encode_display::<A4>(&value, 4, 4), Ok(b"ABCD".to_vec()));
        assert_eq!(value.0.get(), 1);
    }

    #[test]
    fn enums_are_rejected_whatever_their_serde_names() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        enum Code {
            #[serde(rename = "ABCD")]
            Value,
        }

        let mut output = [0u8; 4];
        let mut out = output.as_mut_slice();
        assert_eq!(
            encode_serde_scalar::<_, A4>(&Code::Value, &mut out, &mut [][..]),
            Err(Error::Internal)
        );
        assert_eq!(
            decode_serde_scalar::<Code, A4>(&mut b"ABCD".as_slice(), &mut &mut [][..]),
            Err(Error::Internal)
        );
    }

    mod proptests {
        use proptest::prelude::*;

        use super::*;

        proptest! {
            #[test]
            fn collect_str_preserves_unicode_with_exact_scratch(chars in proptest::collection::vec(any::<char>(), 0..16)) {
                let text: String = chars.into_iter().collect();
                prop_assert_eq!(encode_display::<Text>(&text, text.len(), text.len()), Ok(text.as_bytes().to_vec()));
                if !text.is_empty() {
                    prop_assert_eq!(encode_display::<Text>(&text, text.len(), text.len() - 1), Err(Error::BufferOverflow));
                }
            }
        }
    }
}
