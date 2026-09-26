use serde::Serialize;
use serde::de::value::StrDeserializer;
use serde::de::{DeserializeOwned, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::ser::{Impossible, SerializeMap, SerializeSeq, SerializeTuple, SerializeTupleStruct};

use super::bertlv::{encode_hex_upper, encode_unknown_tlv_from_tag};
use super::*;
use crate::Error;
use crate::primitive::bertlv::{BerTlvEntry, MAX_BER_TAG_BYTES, parse_ber_tag_hex};
use crate::utils::cold_path;

trait BerTlvTextSink {
    type Ok;

    fn accept(self, text: &str) -> Result<Self::Ok, Error>;
}

struct ParseUnknownTagSink;

impl BerTlvTextSink for ParseUnknownTagSink {
    type Ok = ([u8; MAX_BER_TAG_BYTES], usize);

    #[inline(always)]
    fn accept(self, text: &str) -> Result<Self::Ok, Error> {
        parse_ber_tag_hex(text)
    }
}

struct EncodeUnknownValueSink<'a, 'b> {
    output: &'a mut &'b mut [u8],
    tag_bytes: [u8; MAX_BER_TAG_BYTES],
    tag_len: usize,
}

impl BerTlvTextSink for EncodeUnknownValueSink<'_, '_> {
    type Ok = ();

    #[inline(always)]
    fn accept(self, text: &str) -> Result<Self::Ok, Error> {
        let tag = self.tag_bytes.get(..self.tag_len).unwrap_or(&self.tag_bytes);
        encode_unknown_tlv_from_tag(self.output, tag, text)
    }
}

struct BerTlvTextSerializer<'a, S> {
    scratch: &'a mut [u8],
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
    fn collect_str<T: ?Sized + core::fmt::Display>(self, value: &T) -> Result<Self::Ok, Self::Error> {
        use std::io::Write;

        let capacity = self.scratch.len();
        let used = {
            let mut remaining = &mut *self.scratch;
            write!(&mut remaining, "{value}").map_err(|_| {
                cold_path();
                Error::BufferOverflow
            })?;
            capacity - remaining.len()
        };
        let text = self.scratch.get(..used).ok_or_else(|| {
            cold_path();
            Error::Internal
        })?;
        let text = core::str::from_utf8(text).map_err(|_| {
            cold_path();
            Error::Internal
        })?;
        self.sink.accept(text)
    }

    #[inline(always)]
    fn is_human_readable(&self) -> bool {
        true
    }
}

#[inline(always)]
fn parse_unknown_tag_from_serialize<T: ?Sized + Serialize>(
    scratch: &mut [u8],
    value: &T,
) -> Result<([u8; MAX_BER_TAG_BYTES], usize), Error> {
    value.serialize(BerTlvTextSerializer {
        scratch,
        sink: ParseUnknownTagSink,
    })
}

#[inline(always)]
fn encode_unknown_value_from_serialize<T: ?Sized + Serialize>(
    output: &mut &mut [u8],
    scratch: &mut [u8],
    tag_bytes: [u8; MAX_BER_TAG_BYTES],
    tag_len: usize,
    value: &T,
) -> Result<(), Error> {
    value.serialize(BerTlvTextSerializer {
        scratch,
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

impl<'de> serde::Deserializer<'de> for BerTlvTextDeserializer<'_> {
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
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
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

impl<'de> SeqAccess<'de> for BerTlvPairAccess<'_> {
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

impl<'de> serde::Deserializer<'de> for BerTlvPairDeserializer<'_> {
    type Error = Error;

    #[inline(always)]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let mut access = BerTlvPairAccess {
            key: self.key,
            value: self.value,
            index: 0,
        };
        let value = visitor.visit_seq(&mut access)?;
        if access.index != 2 {
            cold_path();
            return Err(Error::Internal);
        }
        Ok(value)
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

struct BerTlvSeqDeserializer<'a, 'de, const ALLOW_ZERO_PADDING: bool> {
    input: &'a mut &'de [u8],
    scratch: &'a mut &'de mut [u8],
}

impl<'de, const ALLOW_ZERO_PADDING: bool> SeqAccess<'de> for BerTlvSeqDeserializer<'_, 'de, ALLOW_ZERO_PADDING> {
    type Error = Error;

    #[inline(always)]
    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        let Some(entry) = decode_ber_tlv_collection_entry::<ALLOW_ZERO_PADDING>(self.input)? else {
            return Ok(None);
        };
        // The text is copied into owned values, so each entry reuses the same scratch.
        let mut workspace = &mut **self.scratch;
        let key = encode_hex_upper(&mut workspace, entry.tag)?;
        let value = encode_hex_upper(&mut workspace, entry.value)?;
        seed.deserialize(BerTlvPairDeserializer { key, value }).map(Some)
    }
}

struct BerTlvMapDeserializer<'a, 'de, const ALLOW_ZERO_PADDING: bool> {
    input: &'a mut &'de [u8],
    scratch: &'a mut &'de mut [u8],
    pending: Option<BerTlvEntry<'de>>,
}

impl<'de, const ALLOW_ZERO_PADDING: bool> MapAccess<'de> for BerTlvMapDeserializer<'_, 'de, ALLOW_ZERO_PADDING> {
    type Error = Error;

    #[inline(always)]
    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        let Some(entry) = decode_ber_tlv_collection_entry::<ALLOW_ZERO_PADDING>(self.input)? else {
            return Ok(None);
        };
        self.pending = Some(entry);
        // The text is copied into owned values, so each entry reuses the same scratch.
        let key = encode_hex_upper(&mut &mut **self.scratch, entry.tag)?;
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
        let value = encode_hex_upper(&mut &mut **self.scratch, entry.value)?;
        seed.deserialize(BerTlvTextDeserializer { text: value })
    }
}

struct BerTlvDeserializer<'a, 'de, const ALLOW_ZERO_PADDING: bool> {
    input: &'a mut &'de [u8],
    scratch: &'a mut &'de mut [u8],
}

#[inline(always)]
pub(crate) fn decode_ber_tlv_serde<'de, T, const ALLOW_ZERO_PADDING: bool>(
    input: &mut &'de [u8],
    scratch: &mut &'de mut [u8],
) -> Result<T, Error>
where
    T: DeserializeOwned,
{
    T::deserialize(BerTlvDeserializer::<ALLOW_ZERO_PADDING> { input, scratch })
}

impl<'de, const ALLOW_ZERO_PADDING: bool> serde::Deserializer<'de> for BerTlvDeserializer<'_, 'de, ALLOW_ZERO_PADDING> {
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
        visitor.visit_seq(BerTlvSeqDeserializer::<ALLOW_ZERO_PADDING> {
            input: self.input,
            scratch: self.scratch,
        })
    }

    #[inline(always)]
    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let mut access = BerTlvMapDeserializer::<ALLOW_ZERO_PADDING> {
            input: self.input,
            scratch: self.scratch,
            pending: None,
        };
        let value = visitor.visit_map(&mut access)?;
        if access.pending.is_some() {
            cold_path();
            return Err(Error::Internal);
        }
        Ok(value)
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
    scratch: &'a mut [u8],
    pending_tag: Option<([u8; MAX_BER_TAG_BYTES], usize)>,
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
        self.pending_tag = Some(parse_unknown_tag_from_serialize(self.scratch, key)?);
        Ok(())
    }

    #[inline(always)]
    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        let (tag_bytes, tag_len) = self.pending_tag.take().ok_or_else(|| {
            cold_path();
            Error::Internal
        })?;
        encode_unknown_value_from_serialize(self.output, self.scratch, tag_bytes, tag_len, value)
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
    scratch: &'a mut [u8],
}

struct BerTlvPairTupleSerializer<'a, 'b> {
    output: &'a mut &'b mut [u8],
    scratch: &'a mut [u8],
    pending_tag: Option<([u8; MAX_BER_TAG_BYTES], usize)>,
    index: u8,
}

impl BerTlvPairTupleSerializer<'_, '_> {
    #[inline(always)]
    fn serialize_item<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Error> {
        match self.index {
            0 => {
                self.pending_tag = Some(parse_unknown_tag_from_serialize(self.scratch, value)?);
                self.index = 1;
                Ok(())
            }
            1 => {
                let (tag_bytes, tag_len) = self.pending_tag.take().ok_or_else(|| {
                    cold_path();
                    Error::Internal
                })?;
                self.index = 2;
                encode_unknown_value_from_serialize(self.output, self.scratch, tag_bytes, tag_len, value)
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
            scratch: self.scratch,
            pending_tag: None,
            index: 0,
        })
    }

    #[inline(always)]
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        if len.is_some_and(|len| len != 2) {
            cold_path();
            return Err(Error::Internal);
        }
        Ok(BerTlvPairTupleSerializer {
            output: self.output,
            scratch: self.scratch,
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
    fn collect_str<T: ?Sized + core::fmt::Display>(self, _value: &T) -> Result<Self::Ok, Self::Error> {
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
    scratch: &'a mut [u8],
}

impl SerializeSeq for BerTlvSeqSerializer<'_, '_> {
    type Ok = ();
    type Error = Error;

    #[inline(always)]
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        value.serialize(BerTlvPairSerializer {
            output: self.output,
            scratch: self.scratch,
        })
    }

    #[inline(always)]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

struct BerTlvSerializer<'a, 'b> {
    output: &'a mut &'b mut [u8],
    scratch: &'a mut [u8],
}

#[inline(always)]
pub(crate) fn encode_ber_tlv_serde<T: ?Sized + Serialize>(output: &mut &mut [u8], scratch: &mut [u8], value: &T) -> Result<(), Error> {
    value.serialize(BerTlvSerializer { output, scratch })
}

impl<T, const ALLOW_ZERO_PADDING: bool> CompositeFmt<T> for BerTlvList<T, ALLOW_ZERO_PADDING>
where
    T: Serialize + DeserializeOwned,
{
    type Decoded<'de> = T;

    #[inline(always)]
    fn encode(output: &mut &mut [u8], scratch: &mut [u8], value: &T) -> Result<(), CompositeError> {
        encode_ber_tlv_serde(output, scratch, value)?;
        Ok(())
    }

    #[inline(always)]
    fn decode<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<T, CompositeError> {
        let value = decode_ber_tlv_serde::<T, ALLOW_ZERO_PADDING>(input, scratch)?;
        if ALLOW_ZERO_PADDING {
            *input = crate::primitive::text::decode_padded(input, 0, true, 0);
        }
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
        Ok(BerTlvSeqSerializer {
            output: self.output,
            scratch: self.scratch,
        })
    }

    #[inline(always)]
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(BerTlvMapSerializer {
            output: self.output,
            scratch: self.scratch,
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
    fn collect_str<T: ?Sized + core::fmt::Display>(self, _value: &T) -> Result<Self::Ok, Self::Error> {
        cold_path();
        Err(Error::Internal)
    }

    #[inline(always)]
    fn is_human_readable(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use core::fmt;

    use super::*;

    struct Formatted<T>(T);

    impl<T: fmt::Display> Serialize for Formatted<T> {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.collect_str(&self.0)
        }
    }

    fn encode(value: &impl Serialize, scratch_len: usize) -> Result<Vec<u8>, Error> {
        let mut output = [0; 128];
        let capacity = output.len();
        let mut out = output.as_mut_slice();
        let mut storage = [0; 64];
        let scratch = &mut storage[..scratch_len];
        encode_ber_tlv_serde(&mut out, scratch, value)?;
        assert_eq!(scratch.len(), scratch_len);
        let used = capacity - out.len();
        Ok(output[..used].to_vec())
    }

    #[test]
    fn collect_str_uses_reusable_bounded_scratch() {
        let value = [(Formatted("59"), Formatted("ABCD")), (Formatted("59"), Formatted(""))];
        // Keys and values are formatted in turn, reusing scratch: the longest, "ABCD", sets the bound.
        for capacity in [0, 3, 4, 64] {
            let expected = if capacity < 4 {
                Err(Error::BufferOverflow)
            } else {
                Ok(b"\x59\x02\xAB\xCD\x59\0".to_vec())
            };
            assert_eq!(encode(&value.as_slice(), capacity), expected);
        }
        assert_eq!(encode(&[("59", "ABCD")].as_slice(), 0), Ok(b"\x59\x02\xAB\xCD".to_vec()));
        let map = std::collections::BTreeMap::from([("59", Formatted("ABCD"))]);
        assert_eq!(encode(&map, 4), Ok(b"\x59\x02\xAB\xCD".to_vec()));
        assert_eq!(encode(&map, 3), Err(Error::BufferOverflow));
        assert_eq!(encode(&[("59", Formatted(""))].as_slice(), 0), Ok(b"\x59\0".to_vec()));
        assert_eq!(encode(&[(Formatted("bad"), "AB")].as_slice(), 11), Err(Error::Invalid));
        for (value, error) in [
            ("A", Error::InvalidValueLength),
            ("ab", Error::Invalid),
            ("GG", Error::Invalid),
            ("€", Error::InvalidValueLength),
        ] {
            assert_eq!(encode(&[("59", Formatted(value))].as_slice(), 11), Err(error));
        }
        assert_eq!(
            encode_ber_tlv_serde(&mut [0; 3].as_mut_slice(), [0; 11].as_mut_slice(), &map),
            Err(Error::BufferOverflow)
        );
    }

    #[test]
    fn collect_str_formats_once_and_rejects_unsupported_shapes_without_formatting() {
        struct Counted(core::cell::Cell<usize>);
        impl fmt::Display for Counted {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.set(self.0.get() + 1);
                formatter.write_str("AB")?;
                formatter.write_str("CD")
            }
        }
        let counted = Formatted(Counted(core::cell::Cell::new(0)));
        assert_eq!(encode(&[("59", &counted)].as_slice(), 4), Ok(b"\x59\x02\xAB\xCD".to_vec()));
        assert_eq!(counted.0.0.get(), 1);
        assert_eq!(encode(&counted, 4), Err(Error::Internal));
        assert_eq!(encode(&[&counted].as_slice(), 4), Err(Error::Internal));
        assert_eq!(counted.0.0.get(), 1);
    }

    #[test]
    fn pairs_require_two_elements_with_or_without_a_length_hint() {
        struct Pair<'a>(Option<usize>, &'a [&'a str]);
        impl Serialize for Pair<'_> {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                let mut seq = serializer.serialize_seq(self.0)?;
                for value in self.1 {
                    seq.serialize_element(value)?;
                }
                seq.end()
            }
        }
        let elements = ["59", "ABCD", "extra"];
        for hint in [None, Some(0), Some(1), Some(2), Some(3)] {
            for len in 0..=elements.len() {
                let expected = if len == 2 && hint.is_none_or(|n| n == 2) {
                    Ok(b"\x59\x02\xAB\xCD".to_vec())
                } else {
                    Err(Error::Internal)
                };
                assert_eq!(encode(&[Pair(hint, &elements[..len])].as_slice(), 0), expected);
            }
        }
    }

    #[test]
    fn visitors_must_consume_values_but_can_explicitly_ignore_them() {
        use serde::Deserializer;
        use serde::de::IgnoredAny;

        struct ReadEntry(bool);
        impl<'de> Visitor<'de> for ReadEntry {
            type Value = ();
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("one entry")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
                seq.next_element::<String>()?;
                if self.0 {
                    seq.next_element::<IgnoredAny>()?;
                }
                Ok(())
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
                map.next_key::<String>()?;
                if self.0 {
                    map.next_value::<IgnoredAny>()?;
                }
                Ok(())
            }
        }
        for complete in [false, true] {
            let expected = if complete { Ok(()) } else { Err(Error::Internal) };
            let pair = BerTlvPairDeserializer { key: "59", value: "ABCD" };
            assert_eq!(pair.deserialize_seq(ReadEntry(complete)), expected);
            let mut input = b"\x59\x02\xAB\xCD".as_slice();
            let mut scratch = [0; 32];
            let map = BerTlvDeserializer::<false> {
                input: &mut input,
                scratch: &mut scratch.as_mut_slice(),
            };
            assert_eq!(map.deserialize_map(ReadEntry(complete)), expected);
        }
        let values =
            decode_ber_tlv_serde::<Vec<(String, IgnoredAny)>, false>(&mut b"\x59\x02\xAB\xCD".as_slice(), &mut [0; 32].as_mut_slice())
                .unwrap();
        assert_eq!(values.len(), 1);
        assert_eq!(values[0].0, "59");
    }

    #[test]
    fn collection_padding_is_opt_in_and_preserves_values() {
        type Pairs = Vec<(String, String)>;
        type Map = std::collections::BTreeMap<String, String>;
        let wire = b"\0\x59\x02\0\xFF\0\0\xFF\x01\0\0";
        let expected = vec![("59".into(), "00FF".into()), ("FF01".into(), "".into())];
        let decoded = BerTlvList::<Pairs, true>::decode(&mut wire.as_slice(), &mut &mut [0; 64][..]).unwrap();
        assert_eq!(decoded, expected);
        assert_eq!(
            BerTlvList::<Map, true>::decode(&mut wire.as_slice(), &mut &mut [0; 64][..]).unwrap(),
            expected.into_iter().collect()
        );
        assert_eq!(
            BerTlvList::<Pairs>::decode(&mut wire.as_slice(), &mut &mut [0; 64][..])
                .unwrap_err()
                .kind,
            Error::Invalid
        );
        let mut output = [0; 16];
        let mut out = output.as_mut_slice();
        BerTlvList::<Pairs, true>::encode(&mut out, &mut [], &decoded).unwrap();
        let used = 16 - out.len();
        assert_eq!(&output[..used], b"\x59\x02\0\xFF\xFF\x01\0");
        assert!(
            BerTlvList::<Pairs, true>::decode(&mut b"\0\0".as_slice(), &mut &mut [][..])
                .unwrap()
                .is_empty()
        );
        assert!(
            BerTlvList::<Pairs>::decode(&mut b"".as_slice(), &mut &mut [][..])
                .unwrap()
                .is_empty()
        );
        for (bytes, error) in [
            (b"\0\x59\x02\0".as_slice(), Error::UnexpectedEof),
            (b"\0\x59\x80\0", Error::Invalid),
            (b"\0\xFF", Error::UnexpectedEof),
        ] {
            assert_eq!(
                BerTlvList::<Pairs, true>::decode(&mut &*bytes, &mut &mut [0; 64][..])
                    .unwrap_err()
                    .kind,
                error
            );
        }
    }

    #[test]
    fn complete_visitors_allow_only_configured_trailing_padding() {
        #[derive(Debug, PartialEq)]
        struct One((String, String));
        impl Serialize for One {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.collect_seq([&self.0])
            }
        }
        impl<'de> serde::Deserialize<'de> for One {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                struct ReadOne;
                impl<'de> Visitor<'de> for ReadOne {
                    type Value = One;
                    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                        formatter.write_str("one entry")
                    }
                    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<One, A::Error> {
                        seq.next_element()?
                            .map(One)
                            .ok_or_else(|| serde::de::Error::custom("missing entry"))
                    }
                }
                deserializer.deserialize_seq(ReadOne)
            }
        }
        for bytes in [b"\x59\x01\xAB".as_slice(), b"\x59\x01\xAB\0\0"] {
            assert_eq!(
                BerTlvList::<One, true>::decode(&mut &*bytes, &mut &mut [0; 32][..]).unwrap(),
                One(("59".into(), "AB".into()))
            );
        }
        assert_eq!(
            BerTlvList::<One>::decode(&mut b"\x59\x01\xAB\0".as_slice(), &mut &mut [0; 32][..])
                .unwrap_err()
                .kind,
            Error::Invalid
        );
        assert_eq!(
            BerTlvList::<One, true>::decode(&mut b"\x59\x01\xAB\0\x5A\0".as_slice(), &mut &mut [0; 32][..])
                .unwrap_err()
                .kind,
            Error::Invalid
        );
    }

    mod proptests {
        use proptest::prelude::*;

        use super::*;

        proptest! {
            #[test]
            fn padding_is_ignored_only_at_entry_boundaries(bytes in prop::collection::vec(any::<u8>(), 0..32), padding in prop::array::uniform3(0usize..8)) {
                type Pairs = Vec<(String, String)>;
                let mut wire = vec![0; padding[0]];
                wire.extend_from_slice(&[0x59, bytes.len() as u8]);
                wire.extend_from_slice(&bytes);
                wire.extend(std::iter::repeat_n(0, padding[1]));
                wire.extend_from_slice(&[0x59, 0]);
                wire.extend(std::iter::repeat_n(0, padding[2]));
                let hex: String = bytes.iter().map(|byte| format!("{byte:02X}")).collect();
                let expected = vec![("59".into(), hex), ("59".into(), "".into())];
                let mut input = wire.as_slice();
                let mut scratch = [0;128];
                prop_assert_eq!(BerTlvList::<Pairs, true>::decode(&mut input, &mut &mut scratch[..]).unwrap(), expected.clone());
                prop_assert!(input.is_empty());
                let strict = BerTlvList::<Pairs>::decode(&mut wire.as_slice(), &mut &mut [0;128][..]).map_err(|error| error.kind);
                prop_assert_eq!(strict, if padding == [0;3] { Ok(expected) } else { Err(Error::Invalid) });
            }

            #[test]
            fn formatted_hex_matches_plain_strings(value in "([0-9A-F]{2}){0,16}") {
                let capacity = 2.max(value.len());
                let formatted = [(Formatted("59"), Formatted(value.as_str()))];
                let plain = [("59", value.as_str())];
                prop_assert_eq!(encode(&formatted.as_slice(), capacity), encode(&plain.as_slice(), 0));
                prop_assert_eq!(encode(&formatted.as_slice(), capacity - 1), Err(Error::BufferOverflow));
            }
        }
    }
}
