use core::marker::PhantomData;

use serde::Serialize;
use serde::de::value::StrDeserializer;
use serde::de::{DeserializeOwned, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::ser::{Impossible, SerializeMap, SerializeSeq, SerializeTuple, SerializeTupleStruct};

use super::*;
use crate::Error;
use crate::primitive::bertlv::{
    BerTlvEntry, decode_ber_tlv_entry, encode_hex_upper_scratch, encode_unknown_tag_key_scratch, encode_unknown_tlv_from_tag,
    parse_unknown_tag_key,
};
use crate::utils::cold_path;

trait BerTlvTextSink {
    type Ok;

    fn accept(self, text: &str) -> Result<Self::Ok, Error>;
}

struct ParseUnknownTagSink;

impl BerTlvTextSink for ParseUnknownTagSink {
    type Ok = ([u8; 4], usize);

    #[inline(always)]
    fn accept(self, text: &str) -> Result<Self::Ok, Error> {
        parse_unknown_tag_key(text)
    }
}

struct EncodeUnknownValueSink<'a, 'b> {
    output: &'a mut &'b mut [u8],
    tag_bytes: [u8; 4],
    tag_len: usize,
}

impl BerTlvTextSink for EncodeUnknownValueSink<'_, '_> {
    type Ok = ();

    #[inline(always)]
    fn accept(self, text: &str) -> Result<Self::Ok, Error> {
        encode_unknown_tlv_from_tag(self.output, &self.tag_bytes[..self.tag_len], text)
    }
}

struct BerTlvTextSerializer<'a, S> {
    _marker: PhantomData<&'a mut [u8]>,
    sink: S,
}

impl<S: BerTlvTextSink> BerTlvTextSerializer<'_, S> {
    #[inline(always)]
    fn encode_char(self, value: char) -> Result<S::Ok, Error> {
        let mut buf = [0u8; 4];
        let text = value.encode_utf8(&mut buf);
        self.sink.accept(text)
    }
}

impl<S: BerTlvTextSink> serde::Serializer for BerTlvTextSerializer<'_, S> {
    type Ok = S::Ok;
    type Error = Error;
    type SerializeSeq = Impossible<S::Ok, Error>;
    type SerializeTuple = Impossible<S::Ok, Error>;
    type SerializeTupleStruct = Impossible<S::Ok, Error>;
    type SerializeTupleVariant = Impossible<S::Ok, Error>;
    type SerializeMap = Impossible<S::Ok, Error>;
    type SerializeStruct = Impossible<S::Ok, Error>;
    type SerializeStructVariant = Impossible<S::Ok, Error>;

    #[inline(always)]
    fn serialize_bool(self, _v: bool) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_i8(self, _v: i8) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_i16(self, _v: i16) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_i32(self, _v: i32) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_i64(self, _v: i64) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_i128(self, _v: i128) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_u8(self, _v: u8) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_u16(self, _v: u16) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_u32(self, _v: u32) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_u64(self, _v: u64) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_u128(self, _v: u128) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        self.encode_char(v)
    }

    #[inline(always)]
    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.sink.accept(v)
    }

    #[inline(always)]
    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    #[inline(always)]
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_unit_variant(self, _name: &'static str, _variant_index: u32, variant: &'static str) -> Result<Self::Ok, Self::Error> {
        self.sink.accept(variant)
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
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_tuple_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeTupleStruct, Self::Error> {
        cold_path();
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
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct, Self::Error> {
        cold_path();
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
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn is_human_readable(&self) -> bool {
        true
    }
}

#[inline(always)]
fn parse_unknown_tag_from_serialize<T: ?Sized + Serialize>(value: &T) -> Result<([u8; 4], usize), Error> {
    value.serialize(BerTlvTextSerializer {
        _marker: PhantomData,
        sink: ParseUnknownTagSink,
    })
}

#[inline(always)]
fn encode_unknown_value_from_serialize<T: ?Sized + Serialize>(
    output: &mut &mut [u8],
    tag_bytes: [u8; 4],
    tag_len: usize,
    value: &T,
) -> Result<(), Error> {
    value.serialize(BerTlvTextSerializer {
        _marker: PhantomData,
        sink: EncodeUnknownValueSink {
            output,
            tag_bytes,
            tag_len,
        },
    })
}

struct BerTlvTextDeserializer<'a> {
    text: &'a str,
}

impl<'de> serde::Deserializer<'de> for BerTlvTextDeserializer<'de> {
    type Error = Error;

    #[inline(always)]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_str(self.text)
    }

    #[inline(always)]
    fn deserialize_bool<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn deserialize_i8<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn deserialize_i16<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn deserialize_i32<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn deserialize_i64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn deserialize_i128<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn deserialize_u8<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn deserialize_u16<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn deserialize_u32<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn deserialize_u64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn deserialize_u128<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn deserialize_f32<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let mut chars = self.text.chars();
        match (chars.next(), chars.next()) {
            (Some(ch), None) => visitor.visit_char(ch),
            _ => {
                cold_path();
                Err(Error::Invalid)
            }
        }
    }

    #[inline(always)]
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_str(self.text)
    }

    #[inline(always)]
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_str(self.text)
    }

    #[inline(always)]
    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        cold_path();
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
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn deserialize_unit_struct<V>(self, _name: &'static str, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        cold_path();
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
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn deserialize_tuple<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn deserialize_tuple_struct<V>(self, _name: &'static str, _len: usize, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn deserialize_map<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn deserialize_struct<V>(self, _name: &'static str, _fields: &'static [&'static str], _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn deserialize_enum<V>(self, _name: &'static str, _variants: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_enum(StrDeserializer::<Error>::new(self.text))
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
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn is_human_readable(&self) -> bool {
        true
    }
}

struct BerTlvPairAccess<'a> {
    key: &'a str,
    value: &'a str,
    index: u8,
}

impl<'de> SeqAccess<'de> for BerTlvPairAccess<'de> {
    type Error = Error;

    #[inline(always)]
    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        if self.index > 1 {
            return Ok(None);
        }
        let value = match self.index {
            0 => {
                self.index = 1;
                seed.deserialize(BerTlvTextDeserializer { text: self.key })
            }
            1 => {
                self.index = 2;
                seed.deserialize(BerTlvTextDeserializer { text: self.value })
            }
            _ => {
                cold_path();
                Err(Error::Internal)
            }
        };
        value.map(Some)
    }

    #[inline(always)]
    fn size_hint(&self) -> Option<usize> {
        Some((2 - self.index.min(2)) as usize)
    }
}

struct BerTlvPairDeserializer<'a> {
    key: &'a str,
    value: &'a str,
}

impl<'de> serde::Deserializer<'de> for BerTlvPairDeserializer<'de> {
    type Error = Error;

    #[inline(always)]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_seq(BerTlvPairAccess {
            key: self.key,
            value: self.value,
            index: 0,
        })
    }

    #[inline(always)]
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    #[inline(always)]
    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if len != 2 {
            cold_path();
            return Err(Error::Internal);
        }
        self.deserialize_any(visitor)
    }

    #[inline(always)]
    fn deserialize_tuple_struct<V>(self, _name: &'static str, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }

    #[inline(always)]
    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    #[inline(always)]
    fn deserialize_bool<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        cold_path();
        Err(Error::Internal)
    }

    serde::forward_to_deserialize_any! {
        i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf
        option unit unit_struct map struct enum identifier ignored_any
    }
}

struct BerTlvSeqDeserializer<'a, 'de> {
    input: &'a mut &'de [u8],
    scratch: &'a mut &'de mut [u8],
}

impl<'de> SeqAccess<'de> for BerTlvSeqDeserializer<'_, 'de> {
    type Error = Error;

    #[inline(always)]
    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        let Some(entry) = decode_ber_tlv_entry(self.input)? else {
            return Ok(None);
        };
        let key = encode_unknown_tag_key_scratch(self.scratch, entry.tag)?;
        let value = encode_hex_upper_scratch(self.scratch, entry.value)?;
        seed.deserialize(BerTlvPairDeserializer { key, value }).map(Some)
    }
}

struct BerTlvMapDeserializer<'a, 'de> {
    input: &'a mut &'de [u8],
    scratch: &'a mut &'de mut [u8],
    pending: Option<BerTlvEntry<'de>>,
}

impl<'de> MapAccess<'de> for BerTlvMapDeserializer<'_, 'de> {
    type Error = Error;

    #[inline(always)]
    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        let Some(entry) = decode_ber_tlv_entry(self.input)? else {
            return Ok(None);
        };
        self.pending = Some(entry);
        let key = encode_unknown_tag_key_scratch(self.scratch, entry.tag)?;
        seed.deserialize(BerTlvTextDeserializer { text: key }).map(Some)
    }

    #[inline(always)]
    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let entry = self.pending.take().ok_or_else(|| {
            cold_path();
            Error::Internal
        })?;
        let value = encode_hex_upper_scratch(self.scratch, entry.value)?;
        seed.deserialize(BerTlvTextDeserializer { text: value })
    }
}

struct BerTlvDeserializer<'a, 'de> {
    input: &'a mut &'de [u8],
    scratch: &'a mut &'de mut [u8],
}

#[inline(always)]
pub(crate) fn decode_ber_tlv_serde<'de, T>(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<T, Error>
where
    T: DeserializeOwned,
{
    T::deserialize(BerTlvDeserializer { input, scratch })
}

impl<'de> serde::Deserializer<'de> for BerTlvDeserializer<'_, 'de> {
    type Error = Error;

    #[inline(always)]
    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_seq(BerTlvSeqDeserializer {
            input: self.input,
            scratch: self.scratch,
        })
    }

    #[inline(always)]
    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_map(BerTlvMapDeserializer {
            input: self.input,
            scratch: self.scratch,
            pending: None,
        })
    }

    #[inline(always)]
    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf
        option unit unit_struct tuple tuple_struct struct enum identifier ignored_any
    }
}

struct BerTlvMapSerializer<'a, 'b> {
    output: &'a mut &'b mut [u8],
    pending_tag: Option<([u8; 4], usize)>,
}

impl SerializeMap for BerTlvMapSerializer<'_, '_> {
    type Ok = ();
    type Error = Error;

    #[inline(always)]
    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), Self::Error> {
        if self.pending_tag.is_some() {
            cold_path();
            return Err(Error::Internal);
        }
        self.pending_tag = Some(parse_unknown_tag_from_serialize(key)?);
        Ok(())
    }

    #[inline(always)]
    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        let (tag_bytes, tag_len) = self.pending_tag.take().ok_or_else(|| {
            cold_path();
            Error::Internal
        })?;
        encode_unknown_value_from_serialize(self.output, tag_bytes, tag_len, value)
    }

    #[inline(always)]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        if self.pending_tag.is_some() {
            cold_path();
            return Err(Error::Internal);
        }
        Ok(())
    }
}

struct BerTlvPairSerializer<'a, 'b> {
    output: &'a mut &'b mut [u8],
}

struct BerTlvPairTupleSerializer<'a, 'b> {
    output: &'a mut &'b mut [u8],
    pending_tag: Option<([u8; 4], usize)>,
    index: u8,
}

impl BerTlvPairTupleSerializer<'_, '_> {
    #[inline(always)]
    fn serialize_item<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Error> {
        match self.index {
            0 => {
                self.pending_tag = Some(parse_unknown_tag_from_serialize(value)?);
                self.index = 1;
                Ok(())
            }
            1 => {
                let (tag_bytes, tag_len) = self.pending_tag.take().ok_or_else(|| {
                    cold_path();
                    Error::Internal
                })?;
                self.index = 2;
                encode_unknown_value_from_serialize(self.output, tag_bytes, tag_len, value)
            }
            _ => {
                cold_path();
                Err(Error::Internal)
            }
        }
    }

    #[inline(always)]
    fn finish(self) -> Result<(), Error> {
        if self.index != 2 || self.pending_tag.is_some() {
            cold_path();
            return Err(Error::Internal);
        }
        Ok(())
    }
}

impl SerializeSeq for BerTlvPairTupleSerializer<'_, '_> {
    type Ok = ();
    type Error = Error;

    #[inline(always)]
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.serialize_item(value)
    }

    #[inline(always)]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.finish()
    }
}

impl SerializeTuple for BerTlvPairTupleSerializer<'_, '_> {
    type Ok = ();
    type Error = Error;

    #[inline(always)]
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.serialize_item(value)
    }

    #[inline(always)]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.finish()
    }
}

impl SerializeTupleStruct for BerTlvPairTupleSerializer<'_, '_> {
    type Ok = ();
    type Error = Error;

    #[inline(always)]
    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.serialize_item(value)
    }

    #[inline(always)]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.finish()
    }
}

impl<'a, 'b> serde::Serializer for BerTlvPairSerializer<'a, 'b> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = BerTlvPairTupleSerializer<'a, 'b>;
    type SerializeTuple = BerTlvPairTupleSerializer<'a, 'b>;
    type SerializeTupleStruct = BerTlvPairTupleSerializer<'a, 'b>;
    type SerializeTupleVariant = Impossible<(), Error>;
    type SerializeMap = Impossible<(), Error>;
    type SerializeStruct = Impossible<(), Error>;
    type SerializeStructVariant = Impossible<(), Error>;

    #[inline(always)]
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        if len != 2 {
            cold_path();
            return Err(Error::Internal);
        }
        Ok(BerTlvPairTupleSerializer {
            output: self.output,
            pending_tag: None,
            index: 0,
        })
    }

    #[inline(always)]
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        if len != Some(2) {
            cold_path();
            return Err(Error::Internal);
        }
        Ok(BerTlvPairTupleSerializer {
            output: self.output,
            pending_tag: None,
            index: 0,
        })
    }

    #[inline(always)]
    fn serialize_tuple_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_tuple(len)
    }

    #[inline(always)]
    fn serialize_newtype_struct<T: ?Sized + Serialize>(self, _name: &'static str, value: &T) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    #[inline(always)]
    fn serialize_bool(self, _v: bool) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_i8(self, _v: i8) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_i16(self, _v: i16) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_i32(self, _v: i32) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_i64(self, _v: i64) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_i128(self, _v: i128) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_u8(self, _v: u8) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_u16(self, _v: u16) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_u32(self, _v: u32) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_u64(self, _v: u64) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_u128(self, _v: u128) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_char(self, _v: char) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_str(self, _v: &str) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_some<T: ?Sized + Serialize>(self, _value: &T) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_unit_variant(self, _name: &'static str, _variant_index: u32, _variant: &'static str) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        cold_path();
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
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct, Self::Error> {
        cold_path();
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
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn is_human_readable(&self) -> bool {
        true
    }
}

struct BerTlvSeqSerializer<'a, 'b> {
    output: &'a mut &'b mut [u8],
}

impl SerializeSeq for BerTlvSeqSerializer<'_, '_> {
    type Ok = ();
    type Error = Error;

    #[inline(always)]
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        value.serialize(BerTlvPairSerializer { output: self.output })
    }

    #[inline(always)]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

struct BerTlvSerializer<'a, 'b> {
    output: &'a mut &'b mut [u8],
}

#[inline(always)]
pub(crate) fn encode_ber_tlv_serde<T: ?Sized + Serialize>(output: &mut &mut [u8], value: &T) -> Result<(), Error> {
    value.serialize(BerTlvSerializer { output })
}

impl<T> CompositeFmt<T> for BerTlvList<T>
where
    T: Serialize + DeserializeOwned,
{
    type Decoded<'de> = T;

    #[inline(always)]
    fn encode_cursor(output: &mut &mut [u8], scratch: &mut &mut [u8], value: &T) -> Result<(), StructError> {
        let _ = scratch;
        encode_ber_tlv_serde(output, value)?;
        Ok(())
    }

    #[inline(always)]
    fn decode_cursor<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<T, StructError> {
        let value = decode_ber_tlv_serde::<T>(input, scratch)?;
        if !input.is_empty() {
            cold_path();
            return Err(Error::Invalid.into());
        }
        Ok(value)
    }
}

impl<'a, 'b> serde::Serializer for BerTlvSerializer<'a, 'b> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = BerTlvSeqSerializer<'a, 'b>;
    type SerializeTuple = Impossible<(), Error>;
    type SerializeTupleStruct = Impossible<(), Error>;
    type SerializeTupleVariant = Impossible<(), Error>;
    type SerializeMap = BerTlvMapSerializer<'a, 'b>;
    type SerializeStruct = Impossible<(), Error>;
    type SerializeStructVariant = Impossible<(), Error>;

    #[inline(always)]
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(BerTlvSeqSerializer { output: self.output })
    }

    #[inline(always)]
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(BerTlvMapSerializer {
            output: self.output,
            pending_tag: None,
        })
    }

    #[inline(always)]
    fn serialize_newtype_struct<T: ?Sized + Serialize>(self, _name: &'static str, value: &T) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    #[inline(always)]
    fn serialize_bool(self, _v: bool) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_i8(self, _v: i8) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_i16(self, _v: i16) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_i32(self, _v: i32) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_i64(self, _v: i64) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_i128(self, _v: i128) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_u8(self, _v: u8) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_u16(self, _v: u16) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_u32(self, _v: u32) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_u64(self, _v: u64) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_u128(self, _v: u128) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_char(self, _v: char) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_str(self, _v: &str) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_some<T: ?Sized + Serialize>(self, _value: &T) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_unit_variant(self, _name: &'static str, _variant_index: u32, _variant: &'static str) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_tuple_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeTupleStruct, Self::Error> {
        cold_path();
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
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct, Self::Error> {
        cold_path();
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
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn is_human_readable(&self) -> bool {
        true
    }
}
