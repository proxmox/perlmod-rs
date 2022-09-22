//! Serde serializer for perl values.

use serde::{ser, Serialize};

use crate::error::Error;
use crate::Value;
use crate::{array, hash, raw_value};

/// Perl [`Value`](crate::Value) serializer.
struct Serializer;

/// Check if the `perlmod::Serializer` is currently being used for serialization.
///
/// External structs can use this to determine whether serializing a `RawValue` containing an xsub
/// will be serialized into perl in a meaningful way.
pub fn is_active() -> bool {
    raw_value::is_enabled()
}

/// Serialize data into a perl [`Value`](crate::Value).
///
/// Note that in theory it should be safe to send such values to different threads as long as their
/// reference count is exactly one.
pub fn to_value<T>(value: &T) -> Result<Value, Error>
where
    T: Serialize,
{
    let _guard = raw_value::guarded(true);
    value.serialize(&mut Serializer)
}

enum SerHashMode {
    Hash(hash::Hash),
    Raw(Option<Value>),
}

/// Serde map & struct serialization helper.
struct SerHash {
    mode: SerHashMode,
    key: Option<Value>,
}

/// Serde sequence serialization helper.
struct SerArray {
    array: array::Array,
}

/// Serde variant serialization helper.
struct SerVariant<T> {
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
        Ok(Value::new_uint(usize::from(v)))
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
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Error> {
        if raw_value::is_enabled() && name == raw_value::NAME && len == 1 {
            Ok(SerHash::raw())
        } else {
            Ok(SerHash::new())
        }
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
            mode: SerHashMode::Hash(hash::Hash::new()),
            key: None,
        }
    }

    fn raw() -> Self {
        Self {
            mode: SerHashMode::Raw(None),
            key: None,
        }
    }

    fn as_mut_hash(&mut self) -> Option<&mut hash::Hash> {
        match &mut self.mode {
            SerHashMode::Hash(hash) => Some(hash),
            _ => None,
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
                self.as_mut_hash()
                    .ok_or_else(|| Error::new("serialize_value called in raw perl value context"))?
                    .insert_by_value(&key, value);
                Ok(())
            }
        }
    }

    fn end(self) -> Result<Value, Error> {
        if self.key.is_some() {
            Error::fail("missing value for key")
        } else {
            match self.mode {
                SerHashMode::Hash(hash) => Ok(Value::new_ref(&hash)),
                _ => Error::fail("raw value serialized as a map instead of a struct"),
            }
        }
    }
}

struct RawValueSerializer;

macro_rules! fail_impossible {
    () => {
        Err(Error::new("bad type serializing raw value"))
    };
}
macro_rules! impossible {
    ($( ($name:ident $ty:ident) )+) => {
        $(
            fn $name(self, _: $ty) -> Result<Value, Error> {
                fail_impossible!()
            }
        )+
    };
}

impl ser::Serializer for RawValueSerializer {
    type Ok = Value;
    type Error = Error;

    type SerializeSeq = ser::Impossible<Value, Error>;
    type SerializeTuple = ser::Impossible<Value, Error>;
    type SerializeTupleStruct = ser::Impossible<Value, Error>;
    type SerializeTupleVariant = ser::Impossible<Value, Error>;
    type SerializeMap = ser::Impossible<Value, Error>;
    type SerializeStruct = ser::Impossible<Value, Error>;
    type SerializeStructVariant = ser::Impossible<Value, Error>;

    impossible! {
        (serialize_bool bool)
        (serialize_i8  i8)
        (serialize_i16 i16)
        (serialize_i32 i32)
        (serialize_i64 i64)
        (serialize_u8  u8)
        (serialize_u16 u16)
        (serialize_u32 u32)
        (serialize_f32 f32)
        (serialize_f64 f64)
        (serialize_char char)
    }

    fn serialize_u64(self, v: u64) -> Result<Value, Error> {
        Ok(unsafe { Value::from_raw_ref(v as *mut crate::ffi::SV) })
    }

    fn serialize_str(self, _: &str) -> Result<Value, Error> {
        fail_impossible!()
    }

    fn serialize_bytes(self, _: &[u8]) -> Result<Value, Error> {
        fail_impossible!()
    }

    fn serialize_none(self) -> Result<Value, Error> {
        fail_impossible!()
    }

    fn serialize_some<T>(self, _value: &T) -> Result<Value, Error>
    where
        T: ?Sized + Serialize,
    {
        fail_impossible!()
    }

    fn serialize_unit(self) -> Result<Value, Error> {
        fail_impossible!()
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Value, Error> {
        fail_impossible!()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Value, Error> {
        fail_impossible!()
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, _value: &T) -> Result<Value, Error>
    where
        T: ?Sized + Serialize,
    {
        fail_impossible!()
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Value, Error>
    where
        T: ?Sized + Serialize,
    {
        fail_impossible!()
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Error> {
        fail_impossible!()
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Error> {
        fail_impossible!()
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Error> {
        fail_impossible!()
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Error> {
        fail_impossible!()
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Error> {
        fail_impossible!()
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Error> {
        fail_impossible!()
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Error> {
        fail_impossible!()
    }
}

impl ser::SerializeStruct for SerHash {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, field: &'static str, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        match &mut self.mode {
            SerHashMode::Hash(hash) => hash.insert(field, value.serialize(&mut Serializer)?),
            SerHashMode::Raw(raw) => {
                if raw.is_some() {
                    return Error::fail("serialize_field called twice in raw context");
                }
                *raw = Some(value.serialize(RawValueSerializer)?);
            }
        }
        Ok(())
    }

    fn end(self) -> Result<Value, Error> {
        match self.mode {
            SerHashMode::Hash(hash) => Ok(Value::new_ref(&hash)),
            SerHashMode::Raw(Some(value)) => Ok(value),
            SerHashMode::Raw(None) => Error::fail("raw value not properly serialized"),
        }
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
        hash.insert(
            variant,
            Value::new_ref(match &inner.mode {
                SerHashMode::Hash(h) => h,
                _ => unreachable!(),
            }),
        );
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
        match &self.inner.mode {
            SerHashMode::Hash(hash) => {
                hash.insert(field, value.serialize(&mut Serializer)?);
                Ok(())
            }
            _ => unreachable!(),
        }
    }

    fn end(self) -> Result<Value, Error> {
        Ok(Value::new_ref(&self.hash))
    }
}
