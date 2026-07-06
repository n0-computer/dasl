//! Serialization.
use std::{
    collections::{BTreeMap, TryReserveError},
    string::ToString,
    vec::Vec,
};

pub use cbor4ii::core::utils::{BufWriter, IoWriter};
use cbor4ii::core::{
    enc::{self, Encode},
    types,
};
use serde::{Serialize, ser};

use super::{CBOR_TAGS_CID, error::EncodeError};
use crate::cid::CID_SERDE_PRIVATE_IDENTIFIER;

/// Serializes a value to a vector.
pub fn to_vec<T>(value: &T) -> Result<Vec<u8>, EncodeError<TryReserveError>>
where
    T: Serialize + ?Sized,
{
    let writer = BufWriter::new(Vec::new());
    let mut serializer = Serializer::new(writer);
    value.serialize(&mut serializer)?;
    Ok(serializer.into_inner().into_inner())
}

/// Serializes a value to a writer.
pub fn to_writer<W, T>(writer: W, value: &T) -> Result<(), EncodeError<std::io::Error>>
where
    W: std::io::Write,
    T: Serialize,
{
    let mut serializer = Serializer::new(IoWriter::new(writer));
    value.serialize(&mut serializer)
}

/// Serializes a Rust type to a DRISL [`Value`][super::Value].
pub fn to_value<T>(source: T) -> Result<super::Value, EncodeError<TryReserveError>>
where
    T: Serialize,
{
    source.serialize(&mut ValueSerializer)
}

/// A structure for serializing Rust values to DRISL.
pub struct Serializer<W> {
    writer: W,
}

impl<W> Serializer<W> {
    /// Creates a new CBOR serializer.
    pub fn new(writer: W) -> Serializer<W> {
        Serializer { writer }
    }

    /// Returns the underlying writer.
    pub fn into_inner(self) -> W {
        self.writer
    }
}

impl<'a, W: enc::Write> serde::Serializer for &'a mut Serializer<W> {
    type Ok = ();
    type Error = EncodeError<W::Error>;

    type SerializeSeq = CollectSeq<'a, W>;
    type SerializeTuple = BoundedCollect<'a, W>;
    type SerializeTupleStruct = BoundedCollect<'a, W>;
    type SerializeTupleVariant = BoundedCollect<'a, W>;
    type SerializeMap = CollectMap<'a, W>;
    type SerializeStruct = CollectMap<'a, W>;
    type SerializeStructVariant = CollectMap<'a, W>;

    #[inline]
    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        v.encode(&mut self.writer)?;
        Ok(())
    }

    #[inline]
    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        v.encode(&mut self.writer)?;
        Ok(())
    }

    #[inline]
    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        v.encode(&mut self.writer)?;
        Ok(())
    }

    #[inline]
    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        v.encode(&mut self.writer)?;
        Ok(())
    }

    #[inline]
    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        v.encode(&mut self.writer)?;
        Ok(())
    }

    #[inline]
    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        v.encode(&mut self.writer)?;
        Ok(())
    }

    #[inline]
    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        v.encode(&mut self.writer)?;
        Ok(())
    }

    #[inline]
    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        v.encode(&mut self.writer)?;
        Ok(())
    }

    #[inline]
    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        v.encode(&mut self.writer)?;
        Ok(())
    }

    #[inline]
    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        // In DRISL floats are always encoded as f64.
        self.serialize_f64(f64::from(v))
    }

    #[inline]
    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        // In DRISL only finite floats are supported.
        if !v.is_finite() {
            Err(EncodeError::Msg(
                "Float must be a finite number, not Infinity or NaN".into(),
            ))
        } else {
            v.encode(&mut self.writer)?;
            Ok(())
        }
    }

    #[inline]
    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        let mut buf = [0; 4];
        self.serialize_str(v.encode_utf8(&mut buf))
    }

    #[inline]
    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        v.encode(&mut self.writer)?;
        Ok(())
    }

    #[inline]
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        types::Bytes(v).encode(&mut self.writer)?;
        Ok(())
    }

    #[inline]
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        types::Null.encode(&mut self.writer)?;
        Ok(())
    }

    #[inline]
    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    #[inline]
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        // The cbor4ii Serde implementation encodes unit as an empty array, for DRISL we encode
        // it as `NULL`.
        types::Null.encode(&mut self.writer)?;
        Ok(())
    }

    #[inline]
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(variant)
    }

    #[inline]
    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        if name == CID_SERDE_PRIVATE_IDENTIFIER {
            value.serialize(&mut CidSerializer(self))
        } else {
            value.serialize(self)
        }
    }

    #[inline]
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        types::Map::bounded(1, &mut self.writer)?;
        variant.encode(&mut self.writer)?;
        value.serialize(self)
    }

    #[inline]
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        let mem_ser = if let Some(len) = len {
            types::Array::bounded(len, &mut self.writer)?;
            None
        } else {
            Some(Serializer::new(BufWriter::new(Vec::new())))
        };
        Ok(CollectSeq {
            ser: self,
            mem_ser,
            count: 0,
        })
    }

    #[inline]
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        types::Array::bounded(len, &mut self.writer)?;
        Ok(BoundedCollect { ser: self })
    }

    #[inline]
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_tuple(len)
    }

    #[inline]
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        types::Map::bounded(1, &mut self.writer)?;
        variant.encode(&mut self.writer)?;
        types::Array::bounded(len, &mut self.writer)?;
        Ok(BoundedCollect { ser: self })
    }

    #[inline]
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(CollectMap::new(self))
    }

    #[inline]
    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        types::Map::bounded(len, &mut self.writer)?;
        Ok(CollectMap::new(self))
    }

    #[inline]
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        types::Map::bounded(1, &mut self.writer)?;
        variant.encode(&mut self.writer)?;
        types::Map::bounded(len, &mut self.writer)?;
        Ok(CollectMap::new(self))
    }

    #[inline]
    fn serialize_i128(self, v: i128) -> Result<Self::Ok, Self::Error> {
        if !(u64::MAX as i128 >= v && -(u64::MAX as i128 + 1) <= v) {
            return Err(EncodeError::Msg(
                "Integer must be within [-u64::MAX-1, u64::MAX] range".into(),
            ));
        }

        v.encode(&mut self.writer)?;
        Ok(())
    }

    #[inline]
    fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> {
        if (u64::MAX as u128) < v {
            return Err(EncodeError::Msg(
                "Unsigned integer must be within [0, u64::MAX] range".into(),
            ));
        }
        v.encode(&mut self.writer)?;
        Ok(())
    }

    #[inline]
    fn is_human_readable(&self) -> bool {
        false
    }
}

/// Struct for implementign SerializeSeq.
pub struct CollectSeq<'a, W> {
    /// The number of elements. This is used in case the number of elements is not known
    /// beforehand.
    count: usize,
    /// An in-memory serializer in case the number of elements is not known beforehand.
    mem_ser: Option<Serializer<BufWriter>>,
    ser: &'a mut Serializer<W>,
}

/// Helper for processing collections.
pub struct BoundedCollect<'a, W> {
    ser: &'a mut Serializer<W>,
}

impl<W: enc::Write> serde::ser::SerializeSeq for CollectSeq<'_, W> {
    type Ok = ();
    type Error = EncodeError<W::Error>;

    #[inline]
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.count += 1;
        if let Some(ser) = self.mem_ser.as_mut() {
            value
                .serialize(&mut *ser)
                .map_err(|_| EncodeError::Msg("List element cannot be serialized".to_string()))
        } else {
            value.serialize(&mut *self.ser)
        }
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        // Data was buffered in order to be able to write out the number of elements before they
        // are serialized.
        if let Some(ser) = self.mem_ser {
            types::Array::bounded(self.count, &mut self.ser.writer)?;
            self.ser.writer.push(&ser.into_inner().into_inner())?;
        }

        Ok(())
    }
}

impl<W: enc::Write> serde::ser::SerializeTuple for BoundedCollect<'_, W> {
    type Ok = ();
    type Error = EncodeError<W::Error>;

    #[inline]
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        value.serialize(&mut *self.ser)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<W: enc::Write> serde::ser::SerializeTupleStruct for BoundedCollect<'_, W> {
    type Ok = ();
    type Error = EncodeError<W::Error>;

    #[inline]
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        value.serialize(&mut *self.ser)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<W: enc::Write> serde::ser::SerializeTupleVariant for BoundedCollect<'_, W> {
    type Ok = ();
    type Error = EncodeError<W::Error>;

    #[inline]
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        value.serialize(&mut *self.ser)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

/// CBOR RFC-7049 specifies a canonical sort order, where keys are sorted by length first. This
/// was later revised with RFC-8949, but we need to stick to the original order to stay compatible
/// with existing data.
/// We first serialize each map entry (the key and the value) into a buffer and then sort those
/// buffers. Once sorted they are written to the actual output.
pub struct CollectMap<'a, W> {
    buffer: BufWriter,
    entries: Vec<Vec<u8>>,
    ser: &'a mut Serializer<W>,
}

impl<'a, W> CollectMap<'a, W>
where
    W: enc::Write,
{
    fn new(ser: &'a mut Serializer<W>) -> Self {
        Self {
            buffer: BufWriter::new(Vec::new()),
            entries: Vec::new(),
            ser,
        }
    }

    fn serialize<T: Serialize + ?Sized>(
        &mut self,
        maybe_key: Option<&'static str>,
        value: &T,
    ) -> Result<(), EncodeError<W::Error>> {
        // Instantiate a new serializer, so that the buffer can be reused.
        let mut mem_serializer = Serializer::new(&mut self.buffer);
        if let Some(key) = maybe_key {
            key.serialize(&mut mem_serializer)
                .map_err(|_| EncodeError::Msg("Struct key cannot be serialized.".to_string()))?;
        }
        value
            .serialize(&mut mem_serializer)
            .map_err(|_| EncodeError::Msg("Struct value cannot be serialized.".to_string()))?;

        self.entries.push(self.buffer.buffer().to_vec());
        self.buffer.clear();

        Ok(())
    }

    fn end(mut self) -> Result<(), EncodeError<W::Error>> {
        // This sorting step makes sure we have the expected order of the keys. Byte-wise
        // comparison over the encoded forms gives us the right order as keys in DRISL are
        // always (text) strings, hence have the same CBOR major type 3. The length of the string
        // is encoded in the prefix bits along with the major type. This means that a shorter string
        // always sorts before a longer string even with the compact length representation.
        self.entries.sort_unstable();
        for entry in self.entries {
            self.ser.writer.push(&entry)?;
        }
        Ok(())
    }
}

impl<W> serde::ser::SerializeMap for CollectMap<'_, W>
where
    W: enc::Write,
{
    type Ok = ();
    type Error = EncodeError<W::Error>;

    #[inline]
    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Self::Error> {
        // The key needs to be add to the buffer without any further operations. Serializing the
        // value will then do the necessary flushing etc.
        let mut mem_serializer = Serializer::new(&mut self.buffer);
        key.serialize(&mut mem_serializer)
            .map_err(|_| EncodeError::Msg("Map key cannot be serialized.".to_string()))?;
        Ok(())
    }

    #[inline]
    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.serialize(None, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        types::Map::bounded(self.entries.len(), &mut self.ser.writer)?;
        self.end()
    }
}

impl<W> serde::ser::SerializeStruct for CollectMap<'_, W>
where
    W: enc::Write,
{
    type Ok = ();
    type Error = EncodeError<W::Error>;

    #[inline]
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.serialize(Some(key), value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.end()
    }
}

impl<W> serde::ser::SerializeStructVariant for CollectMap<'_, W>
where
    W: enc::Write,
{
    type Ok = ();
    type Error = EncodeError<W::Error>;

    #[inline]
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.serialize(Some(key), value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.end()
    }
}

/// Serializer that can serialize a Rust type to a DRISL [`Value`][super::Value].
pub struct ValueSerializer;

impl<'a> serde::Serializer for &'a mut ValueSerializer {
    type Ok = super::Value;
    type Error = EncodeError<TryReserveError>;

    type SerializeSeq = ValueSerializerSeq<'a>;
    type SerializeTuple = ValueSerializerSeq<'a>;
    type SerializeTupleStruct = ValueSerializerSeq<'a>;
    type SerializeTupleVariant = ValueSerializerTupleVariant<'a>;
    type SerializeMap = ValueSerializerMap<'a>;
    type SerializeStruct = ValueSerializerMap<'a>;
    type SerializeStructVariant = ValueSerializerStructVariant<'a>;

    #[inline]
    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(super::Value::Bool(v))
    }
    #[inline]
    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        Ok(super::Value::Integer(v as i128))
    }
    #[inline]
    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        Ok(super::Value::Integer(v as i128))
    }
    #[inline]
    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Ok(super::Value::Integer(v as i128))
    }
    #[inline]
    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(super::Value::Integer(v as i128))
    }
    #[inline]
    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        Ok(super::Value::Integer(v as i128))
    }
    #[inline]
    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        Ok(super::Value::Integer(v as i128))
    }
    #[inline]
    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        Ok(super::Value::Integer(v as i128))
    }
    #[inline]
    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        Ok(super::Value::Integer(v as i128))
    }
    #[inline]
    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.serialize_f64(v.into())
    }
    #[inline]
    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        // In DRISL only finite floats are supported.
        if !v.is_finite() {
            Err(EncodeError::Msg(
                "Float must be a finite number, not Infinity or NaN".into(),
            ))
        } else {
            Ok(super::Value::Float(v))
        }
    }

    #[inline]
    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        Ok(super::Value::Text(v.into()))
    }

    #[inline]
    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(super::Value::Text(v.into()))
    }

    #[inline]
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(super::Value::Bytes(v.into()))
    }

    #[inline]
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(super::Value::Null)
    }

    #[inline]
    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    #[inline]
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(super::Value::Null)
    }

    #[inline]
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(variant)
    }

    #[inline]
    fn serialize_newtype_struct<T>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        if name == CID_SERDE_PRIVATE_IDENTIFIER {
            let mut bytes = BufWriter::new(Vec::new());
            value.serialize(&mut CidSerializer(&mut Serializer::new(&mut bytes)))?;
            Ok(super::Value::Bytes(bytes.into_inner()))
        } else {
            value.serialize(self)
        }
    }

    #[inline]
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let mut map = BTreeMap::new();
        map.insert(variant.to_string(), value.serialize(self)?);
        Ok(super::Value::Map(map))
    }

    #[inline]
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(ValueSerializerSeq {
            seq: Vec::with_capacity(len.unwrap_or(0)),
            serializer: self,
        })
    }

    #[inline]
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    #[inline]
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_tuple(len)
    }

    #[inline]
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(ValueSerializerTupleVariant {
            variant_name: variant,
            seq: Vec::with_capacity(len),
            serializer: self,
        })
    }

    #[inline]
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(ValueSerializerMap {
            serializer: self,
            map: BTreeMap::new(),
            staged_key: None,
        })
    }

    #[inline]
    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(ValueSerializerMap {
            serializer: self,
            map: BTreeMap::new(),
            staged_key: None,
        })
    }

    #[inline]
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(ValueSerializerStructVariant {
            map: BTreeMap::new(),
            serializer: self,
            variant_name: variant,
        })
    }
}

pub struct ValueSerializerSeq<'a> {
    serializer: &'a mut ValueSerializer,
    seq: Vec<super::Value>,
}

impl serde::ser::SerializeSeq for ValueSerializerSeq<'_> {
    type Ok = super::Value;
    type Error = EncodeError<TryReserveError>;
    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.seq.push(value.serialize(&mut *self.serializer)?);
        Ok(())
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(super::Value::Array(self.seq))
    }
}

impl serde::ser::SerializeTuple for ValueSerializerSeq<'_> {
    type Ok = super::Value;
    type Error = EncodeError<TryReserveError>;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.seq.push(value.serialize(&mut *self.serializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(super::Value::Array(self.seq))
    }
}

impl serde::ser::SerializeTupleStruct for ValueSerializerSeq<'_> {
    type Ok = super::Value;
    type Error = EncodeError<TryReserveError>;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.seq.push(value.serialize(&mut *self.serializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(super::Value::Array(self.seq))
    }
}

pub struct ValueSerializerTupleVariant<'a> {
    variant_name: &'static str,
    serializer: &'a mut ValueSerializer,
    seq: Vec<super::Value>,
}

impl serde::ser::SerializeTupleVariant for ValueSerializerTupleVariant<'_> {
    type Ok = super::Value;
    type Error = EncodeError<TryReserveError>;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.seq.push(value.serialize(&mut *self.serializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let mut map = BTreeMap::new();
        map.insert(self.variant_name.into(), super::Value::Array(self.seq));
        Ok(super::Value::Map(map))
    }
}

pub struct ValueSerializerMap<'a> {
    serializer: &'a mut ValueSerializer,
    map: BTreeMap<String, super::Value>,
    staged_key: Option<String>,
}
impl serde::ser::SerializeMap for ValueSerializerMap<'_> {
    type Ok = super::Value;
    type Error = EncodeError<TryReserveError>;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.staged_key = Some(key.serialize(StringKeySerializer)?);
        Ok(())
    }
    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let key = self.staged_key.take().unwrap();
        self.map
            .insert(key, value.serialize(&mut *self.serializer)?);
        Ok(())
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(super::Value::Map(self.map))
    }
}
impl serde::ser::SerializeStruct for ValueSerializerMap<'_> {
    type Ok = super::Value;
    type Error = EncodeError<TryReserveError>;
    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.map
            .insert(key.into(), value.serialize(&mut *self.serializer)?);
        Ok(())
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(super::Value::Map(self.map))
    }
}

pub struct ValueSerializerStructVariant<'a> {
    variant_name: &'static str,
    serializer: &'a mut ValueSerializer,
    map: BTreeMap<String, super::Value>,
}
impl serde::ser::SerializeStructVariant for ValueSerializerStructVariant<'_> {
    type Ok = super::Value;
    type Error = EncodeError<TryReserveError>;
    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.map
            .insert(key.into(), value.serialize(&mut *self.serializer)?);
        Ok(())
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        let mut wrapper = BTreeMap::new();
        wrapper.insert(self.variant_name.into(), super::Value::Map(self.map));
        Ok(super::Value::Map(wrapper))
    }
}

macro_rules! error_for_primitive_serialize_impls {
    ($error:expr, $($impl_tys:ty, $impls:ident),+ $(,)?) => {
        $(
            fn $impls(self, _value: $impl_tys) -> Result<Self::Ok, Self::Error> {
                Err(ser::Error::custom($error))
            }
        )*
    }
}

static STRING_KEY_ERROR: &str = "invalid key type, expected string";
struct StringKeySerializer;
impl ser::Serializer for StringKeySerializer {
    type Ok = String;
    type Error = EncodeError<TryReserveError>;

    type SerializeSeq = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTuple = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTupleStruct = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTupleVariant = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeMap = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeStruct = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeStructVariant = ser::Impossible<Self::Ok, Self::Error>;

    error_for_primitive_serialize_impls!(
        STRING_KEY_ERROR,
        bool,
        serialize_bool,
        i8,
        serialize_i8,
        i16,
        serialize_i16,
        i32,
        serialize_i32,
        i64,
        serialize_i64,
        u8,
        serialize_u8,
        u16,
        serialize_u16,
        u32,
        serialize_u32,
        u64,
        serialize_u64,
        f32,
        serialize_f32,
        f64,
        serialize_f64,
        char,
        serialize_char,
        &[u8],
        serialize_bytes,
    );

    fn serialize_str(self, value: &str) -> Result<Self::Ok, Self::Error> {
        Ok(value.into())
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom(STRING_KEY_ERROR))
    }
    fn serialize_some<T: ?Sized + ser::Serialize>(
        self,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom(STRING_KEY_ERROR))
    }
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom(STRING_KEY_ERROR))
    }
    fn serialize_unit_struct(self, _name: &str) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom(STRING_KEY_ERROR))
    }
    fn serialize_unit_variant(
        self,
        _name: &str,
        _variant_index: u32,
        _variant: &str,
    ) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom(STRING_KEY_ERROR))
    }

    fn serialize_newtype_struct<T: ?Sized + ser::Serialize>(
        self,
        _name: &str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom(STRING_KEY_ERROR))
    }
    fn serialize_newtype_variant<T: ?Sized + ser::Serialize>(
        self,
        _name: &str,
        _variant_index: u32,
        _variant: &str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom(STRING_KEY_ERROR))
    }
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Err(ser::Error::custom(STRING_KEY_ERROR))
    }
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Err(ser::Error::custom(STRING_KEY_ERROR))
    }
    fn serialize_tuple_struct(
        self,
        _name: &str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(ser::Error::custom(STRING_KEY_ERROR))
    }
    fn serialize_tuple_variant(
        self,
        _name: &str,
        _variant_index: u32,
        _variant: &str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(ser::Error::custom(STRING_KEY_ERROR))
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(ser::Error::custom(STRING_KEY_ERROR))
    }
    fn serialize_struct(
        self,
        _name: &str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Err(ser::Error::custom(STRING_KEY_ERROR))
    }
    fn serialize_struct_variant(
        self,
        _name: &str,
        _variant_index: u32,
        _variant: &str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(ser::Error::custom(STRING_KEY_ERROR))
    }
}

/// Serializing a CID correctly as DRISL.
struct CidSerializer<'a, W>(&'a mut Serializer<W>);

impl<'a, W: enc::Write> ser::Serializer for &'a mut CidSerializer<'a, W>
where
    W::Error: core::fmt::Debug,
{
    type Ok = ();
    type Error = EncodeError<W::Error>;

    type SerializeSeq = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTuple = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTupleStruct = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTupleVariant = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeMap = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeStruct = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeStructVariant = ser::Impossible<Self::Ok, Self::Error>;

    fn serialize_bool(self, _value: bool) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_i8(self, _value: i8) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_i16(self, _value: i16) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_i32(self, _value: i32) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_i64(self, _value: i64) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_u8(self, _value: u8) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_u16(self, _value: u16) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_u32(self, _value: u32) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_u64(self, _value: u64) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_f32(self, _value: f32) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_f64(self, _value: f64) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_char(self, _value: char) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_str(self, _value: &str) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<Self::Ok, Self::Error> {
        // CIDs are serialized with CBOR tag 42.
        types::Tag(CBOR_TAGS_CID as _, types::Bytes(value)).encode(&mut self.0.writer)?;
        Ok(())
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_some<T: ?Sized + ser::Serialize>(
        self,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_unit_struct(self, _name: &str) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_unit_variant(
        self,
        _name: &str,
        _variant_index: u32,
        _variant: &str,
    ) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }

    fn serialize_newtype_struct<T: ?Sized + ser::Serialize>(
        self,
        _name: &str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_newtype_variant<T: ?Sized + ser::Serialize>(
        self,
        _name: &str,
        _variant_index: u32,
        _variant: &str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_tuple_struct(
        self,
        _name: &str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_tuple_variant(
        self,
        _name: &str,
        _variant_index: u32,
        _variant: &str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_struct(
        self,
        _name: &str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_struct_variant(
        self,
        _name: &str,
        _variant_index: u32,
        _variant: &str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
}
