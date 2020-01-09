use serde::{ser, Serialize};

use crate::error::Error;
use crate::Value;
use crate::{array, hash};

pub struct Serializer;

pub fn to_value<T>(value: &T) -> Result<Value, Error>
where
    T: Serialize,
{
    value.serialize(&mut Serializer)
}

pub struct SerHash {
    hash: hash::Hash,
    key: Option<Value>,
}

pub struct SerArray {
    array: array::Array,
}

pub struct SerVariant<T> {
    hash: hash::Hash,
    inner: T,
}

impl<'a> ser::Serializer for &'a mut Serializer {
    type Ok = Value;
    type Error = Error;

    type SerializeSeq = SerArray;
    type SerializeTuple = SerArray;
    type SerializeTupleStruct = SerArray;
    type SerializeTupleVariant = SerVariant<SerArray>;
    type SerializeMap = SerHash;
    type SerializeStruct = SerHash;
    type SerializeStructVariant = SerVariant<SerHash>;

    fn serialize_bool(self, v: bool) -> Result<Value, Error> {
        Ok(Value::new_uint(if v { 1 } else { 0 }))
    }

    fn serialize_i8(self, v: i8) -> Result<Value, Error> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i16(self, v: i16) -> Result<Value, Error> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i32(self, v: i32) -> Result<Value, Error> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i64(self, v: i64) -> Result<Value, Error> {
        Ok(Value::new_int(v as isize))
    }

    fn serialize_u8(self, v: u8) -> Result<Value, Error> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u16(self, v: u16) -> Result<Value, Error> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u32(self, v: u32) -> Result<Value, Error> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u64(self, v: u64) -> Result<Value, Error> {
        Ok(Value::new_uint(v as usize))
    }

    fn serialize_f32(self, v: f32) -> Result<Value, Error> {
        self.serialize_f64(f64::from(v))
    }

    fn serialize_f64(self, v: f64) -> Result<Value, Error> {
        Ok(Value::new_float(v))
    }

    fn serialize_char(self, v: char) -> Result<Value, Error> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<Value, Error> {
        Ok(Value::new_string(v))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Value, Error> {
        Ok(Value::new_bytes(v))
    }

    fn serialize_none(self) -> Result<Value, Error> {
        self.serialize_unit()
    }

    fn serialize_some<T>(self, value: &T) -> Result<Value, Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Value, Error> {
        Ok(Value::new_undef())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Value, Error> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Value, Error> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<Value, Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Value, Error>
    where
        T: ?Sized + Serialize,
    {
        let value = value.serialize(&mut Serializer)?;
        let hash = hash::Hash::new();
        hash.insert(variant, value);
        Ok(Value::from(hash))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Error> {
        Ok(SerArray::new(len))
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Error> {
        Ok(SerVariant::<SerArray>::new(variant, Some(len)))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Error> {
        Ok(SerHash::new())
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Error> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Error> {
        Ok(SerVariant::<SerHash>::new(variant))
    }
}

impl SerArray {
    fn new(len: Option<usize>) -> Self {
        let array = array::Array::new();
        if let Some(len) = len {
            array.reserve(len);
        }
        Self { array }
    }
}

impl ser::SerializeSeq for SerArray {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        self.array.push(value.serialize(&mut Serializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value, Error> {
        Ok(Value::new_ref(&self.array))
    }
}

impl ser::SerializeTuple for SerArray {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        self.array.push(value.serialize(&mut Serializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value, Error> {
        Ok(Value::new_ref(&self.array))
    }
}

impl ser::SerializeTupleStruct for SerArray {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        self.array.push(value.serialize(&mut Serializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value, Error> {
        Ok(Value::new_ref(&self.array))
    }
}

impl SerHash {
    fn new() -> Self {
        Self {
            hash: hash::Hash::new(),
            key: None,
        }
    }
}

impl ser::SerializeMap for SerHash {
    type Ok = Value;
    type Error = Error;

    fn serialize_key<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        if self.key.is_some() {
            Error::fail("serialize_key called twice")
        } else {
            self.key = Some(value.serialize(&mut Serializer)?);
            Ok(())
        }
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        match self.key.take() {
            None => Error::fail("serialize_value called without key"),
            Some(key) => {
                let value = value.serialize(&mut Serializer)?;
                self.hash.insert_by_value(&key, value);
                Ok(())
            }
        }
    }

    fn end(self) -> Result<Value, Error> {
        if self.key.is_some() {
            Error::fail("missing value for key")
        } else {
            Ok(Value::new_ref(&self.hash))
        }
    }
}

impl ser::SerializeStruct for SerHash {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, field: &'static str, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        self.hash.insert(field, value.serialize(&mut Serializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value, Error> {
        Ok(Value::new_ref(&self.hash))
    }
}

impl SerVariant<SerArray> {
    fn new(variant: &str, len: Option<usize>) -> Self {
        let inner = SerArray::new(len);
        let hash = hash::Hash::new();
        hash.insert(variant, Value::new_ref(&inner.array));
        Self { hash, inner }
    }
}

impl ser::SerializeTupleVariant for SerVariant<SerArray> {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        self.inner.array.push(value.serialize(&mut Serializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value, Error> {
        Ok(Value::new_ref(&self.inner.array))
    }
}

impl SerVariant<SerHash> {
    fn new(variant: &str) -> Self {
        let inner = SerHash::new();
        let hash = hash::Hash::new();
        hash.insert(variant, Value::new_ref(&inner.hash));
        Self { hash, inner }
    }
}

impl ser::SerializeStructVariant for SerVariant<SerHash> {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, field: &'static str, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        self.inner
            .hash
            .insert(field, value.serialize(&mut Serializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value, Error> {
        Ok(Value::new_ref(&self.hash))
    }
}
