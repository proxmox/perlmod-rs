//! This is a private module which handles the serialization of return values from exported subs.
//!
//! `perlmod-macro` will serialize to this type in the `ReturnType::Single` case and dynamically
//! decide how many values to return in order to allow returning *lists*, not just array
//! references.

use std::cell::RefCell;

use serde::{Serialize, ser};

use crate::Value;
use crate::error::Error;

use super::Serializer;

thread_local!(static SERIALIZE_LIST: RefCell<bool> = const { RefCell::new(false) });

pub(crate) struct ListGuard(bool);

impl Drop for ListGuard {
    fn drop(&mut self) {
        SERIALIZE_LIST.with(|list| *list.borrow_mut() = self.0);
    }
}

#[inline]
pub(crate) fn guarded(on: bool) -> ListGuard {
    SERIALIZE_LIST.with(move |list| ListGuard(list.replace(on)))
}

#[inline]
pub(crate) fn is_enabled() -> bool {
    SERIALIZE_LIST.with(|list| *list.borrow())
}

/// Wrapper type allowing to choose the way sequences and tuples should be treated in return
/// values.
pub enum Return<T, U> {
    /// Return nothing.
    Void,

    /// This will behave just like `T` does, serializing sequences as array references.
    Single(T),

    /// In this case, the following serde types will serialize to multiple return values:
    /// - Sequence / Array
    /// - Tuple
    /// - Tuple Struct
    /// - Tuple Variants
    ///
    /// Other types will produce the same result as a single value.
    List(U),
}

impl<T, U> serde::Serialize for Return<T, U>
where
    T: serde::Serialize,
    U: serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        match self {
            Self::Void => serializer.serialize_unit(),
            Self::Single(inner) => inner.serialize(serializer),
            Self::List(inner) => {
                let _guard = guarded(true);
                inner.serialize(serializer)
            }
        }
    }
}

/// This type encodes whether a returned value is a single value or a list.
pub enum ReturnValue {
    Single(Value),
    List(Vec<Value>),
}

impl ReturnValue {
    #[doc(hidden)]
    pub fn __private_push_to_stack(self) {
        use crate::ffi;
        match self {
            Self::Single(value) => unsafe {
                ffi::stack_push_raw(value.into_mortal().into_raw());
            },
            Self::List(list) => unsafe {
                ffi::RSPL_stack_resize_by(isize::try_from(list.len()).expect("huge list returned"));
                let mut sp = ffi::RSPL_stack_sp().sub(list.len());
                for value in list {
                    sp = sp.add(1);
                    *sp = value.into_mortal().into_raw();
                }
            },
        }
    }
}

/// A serializer producing a [`ReturnValue`].
pub(super) struct ReturnValueSerializer;

pub(super) struct MakeSingle<T>(T);

pub(super) enum SerList {
    Single(<Serializer as ser::Serializer>::SerializeSeq),
    List(Vec<Value>),
}

pub(super) enum SerTupleVariant {
    Single(<Serializer as ser::Serializer>::SerializeTupleVariant),
    List(Vec<Value>),
}

impl ser::Serializer for ReturnValueSerializer {
    type Ok = ReturnValue;
    type Error = Error;

    type SerializeSeq = SerList;
    type SerializeTuple = SerList;
    type SerializeTupleStruct = SerList;
    type SerializeTupleVariant = SerTupleVariant;
    type SerializeMap = MakeSingle<<Serializer as ser::Serializer>::SerializeMap>;
    type SerializeStruct = MakeSingle<<Serializer as ser::Serializer>::SerializeStruct>;
    type SerializeStructVariant =
        MakeSingle<<Serializer as ser::Serializer>::SerializeStructVariant>;

    fn serialize_bool(self, v: bool) -> Result<ReturnValue, Error> {
        Serializer.serialize_bool(v).map(ReturnValue::Single)
    }

    fn serialize_i8(self, v: i8) -> Result<ReturnValue, Error> {
        Serializer.serialize_i8(v).map(ReturnValue::Single)
    }

    fn serialize_i16(self, v: i16) -> Result<ReturnValue, Error> {
        Serializer.serialize_i16(v).map(ReturnValue::Single)
    }

    fn serialize_i32(self, v: i32) -> Result<ReturnValue, Error> {
        Serializer.serialize_i32(v).map(ReturnValue::Single)
    }

    fn serialize_i64(self, v: i64) -> Result<ReturnValue, Error> {
        Serializer.serialize_i64(v).map(ReturnValue::Single)
    }

    fn serialize_u8(self, v: u8) -> Result<ReturnValue, Error> {
        Serializer.serialize_u8(v).map(ReturnValue::Single)
    }

    fn serialize_u16(self, v: u16) -> Result<ReturnValue, Error> {
        Serializer.serialize_u16(v).map(ReturnValue::Single)
    }

    fn serialize_u32(self, v: u32) -> Result<ReturnValue, Error> {
        Serializer.serialize_u32(v).map(ReturnValue::Single)
    }

    fn serialize_u64(self, v: u64) -> Result<ReturnValue, Error> {
        Serializer.serialize_u64(v).map(ReturnValue::Single)
    }

    fn serialize_f32(self, v: f32) -> Result<ReturnValue, Error> {
        Serializer.serialize_f32(v).map(ReturnValue::Single)
    }

    fn serialize_f64(self, v: f64) -> Result<ReturnValue, Error> {
        Serializer.serialize_f64(v).map(ReturnValue::Single)
    }

    fn serialize_char(self, v: char) -> Result<ReturnValue, Error> {
        Serializer.serialize_char(v).map(ReturnValue::Single)
    }

    fn serialize_str(self, v: &str) -> Result<ReturnValue, Error> {
        Serializer.serialize_str(v).map(ReturnValue::Single)
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<ReturnValue, Error> {
        Serializer.serialize_bytes(v).map(ReturnValue::Single)
    }

    fn serialize_none(self) -> Result<ReturnValue, Error> {
        Serializer.serialize_none().map(ReturnValue::Single)
    }

    fn serialize_some<T>(self, value: &T) -> Result<ReturnValue, Error>
    where
        T: ?Sized + Serialize,
    {
        Serializer.serialize_some(value).map(ReturnValue::Single)
    }

    fn serialize_unit(self) -> Result<ReturnValue, Error> {
        Serializer.serialize_unit().map(ReturnValue::Single)
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<ReturnValue, Error> {
        Serializer
            .serialize_unit_struct(name)
            .map(ReturnValue::Single)
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<ReturnValue, Error> {
        Serializer
            .serialize_unit_variant(name, variant_index, variant)
            .map(ReturnValue::Single)
    }

    fn serialize_newtype_struct<T>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<ReturnValue, Error>
    where
        T: ?Sized + Serialize,
    {
        Serializer
            .serialize_newtype_struct(name, value)
            .map(ReturnValue::Single)
    }

    fn serialize_newtype_variant<T>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<ReturnValue, Error>
    where
        T: ?Sized + Serialize,
    {
        Serializer
            .serialize_newtype_variant(name, variant_index, variant, value)
            .map(ReturnValue::Single)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Error> {
        if is_enabled() {
            Ok(SerList::List(match len {
                Some(len) => Vec::with_capacity(len),
                None => Vec::new(),
            }))
        } else {
            Serializer.serialize_seq(len).map(SerList::Single)
        }
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
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Error> {
        if is_enabled() {
            Ok(SerTupleVariant::List(Vec::with_capacity(len)))
        } else {
            Serializer
                .serialize_tuple_variant(name, variant_index, variant, len)
                .map(SerTupleVariant::Single)
        }
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Error> {
        Serializer.serialize_map(len).map(MakeSingle)
    }

    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Error> {
        Serializer.serialize_struct(name, len).map(MakeSingle)
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Error> {
        Serializer
            .serialize_struct_variant(name, variant_index, variant, len)
            .map(MakeSingle)
    }
}

impl<S> ser::SerializeMap for MakeSingle<S>
where
    S: ser::SerializeMap<Ok = Value, Error = Error>,
{
    type Ok = ReturnValue;
    type Error = Error;

    fn serialize_key<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        self.0.serialize_key(value)
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        self.0.serialize_value(value)
    }

    fn end(self) -> Result<ReturnValue, Error> {
        self.0.end().map(ReturnValue::Single)
    }
}

impl<S> ser::SerializeStruct for MakeSingle<S>
where
    S: ser::SerializeStruct<Ok = Value, Error = Error>,
{
    type Ok = ReturnValue;
    type Error = Error;

    fn serialize_field<T>(&mut self, field: &'static str, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        self.0.serialize_field(field, value)
    }

    fn end(self) -> Result<ReturnValue, Error> {
        self.0.end().map(ReturnValue::Single)
    }
}

impl<S> ser::SerializeStructVariant for MakeSingle<S>
where
    S: ser::SerializeStructVariant<Ok = Value, Error = Error>,
{
    type Ok = ReturnValue;
    type Error = Error;

    fn serialize_field<T>(&mut self, field: &'static str, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        self.0.serialize_field(field, value)
    }

    fn end(self) -> Result<ReturnValue, Error> {
        self.0.end().map(ReturnValue::Single)
    }
}

impl ser::SerializeSeq for SerList {
    type Ok = ReturnValue;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        match self {
            Self::Single(inner) => inner.serialize_element(value),
            Self::List(list) => {
                list.push(value.serialize(Serializer)?);
                Ok(())
            }
        }
    }

    fn end(self) -> Result<ReturnValue, Error> {
        match self {
            Self::Single(inner) => inner.end().map(ReturnValue::Single),
            Self::List(list) => Ok(ReturnValue::List(list)),
        }
    }
}

impl ser::SerializeTuple for SerList {
    type Ok = ReturnValue;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        match self {
            Self::Single(inner) => inner.serialize_element(value),
            Self::List(list) => {
                list.push(value.serialize(Serializer)?);
                Ok(())
            }
        }
    }

    fn end(self) -> Result<ReturnValue, Error> {
        match self {
            Self::Single(inner) => inner.end().map(ReturnValue::Single),
            Self::List(list) => Ok(ReturnValue::List(list)),
        }
    }
}

impl ser::SerializeTupleStruct for SerList {
    type Ok = ReturnValue;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        match self {
            Self::Single(inner) => inner.serialize_field(value),
            Self::List(list) => {
                list.push(value.serialize(Serializer)?);
                Ok(())
            }
        }
    }

    fn end(self) -> Result<ReturnValue, Error> {
        match self {
            Self::Single(inner) => inner.end().map(ReturnValue::Single),
            Self::List(list) => Ok(ReturnValue::List(list)),
        }
    }
}

impl ser::SerializeTupleVariant for SerTupleVariant {
    type Ok = ReturnValue;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        match self {
            Self::Single(inner) => inner.serialize_field(value),
            Self::List(list) => {
                list.push(value.serialize(Serializer)?);
                Ok(())
            }
        }
    }

    fn end(self) -> Result<ReturnValue, Error> {
        match self {
            Self::Single(inner) => inner.end().map(ReturnValue::Single),
            Self::List(list) => Ok(ReturnValue::List(list)),
        }
    }
}
