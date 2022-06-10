//! Serde deserializer for perl values.

use std::marker::PhantomData;

use serde::de::value::BorrowedStrDeserializer;
use serde::de::{
    self, Deserialize, DeserializeSeed, IntoDeserializer, MapAccess, SeqAccess, Visitor,
};

use crate::error::Error;
use crate::raw_value;
use crate::scalar::Type;
use crate::Value;
use crate::{array, ffi, hash};

/// Perl [`Value`](crate::Value) deserializer.
struct Deserializer<'de> {
    input: Value,
    option_allowed: bool,
    _lifetime: PhantomData<&'de Value>,
}

/// Deserialize a perl [`Value`](crate::Value).
///
/// Note that this causes all the underlying data to be copied recursively, except for other
/// [`Value`](crate::Value) variables, which will be references.
pub fn from_value<T>(input: Value) -> Result<T, Error>
where
    T: serde::de::DeserializeOwned,
{
    let _guard = raw_value::guarded(true);
    let mut deserializer = Deserializer::<'static>::from_value(input);
    let out = T::deserialize(&mut deserializer)?;
    Ok(out)
}

/// Deserialize a reference to a perl [`Value`](crate::Value).
///
/// Note that this causes all the underlying data to be copied recursively, except for other
/// [`Value`](crate::Value) variables or `&[u8]` or `&str` types, which will reference the
/// "original" value (whatever that means for perl).
pub fn from_ref_value<'de, T>(input: &'de Value) -> Result<T, Error>
where
    T: Deserialize<'de>,
{
    let _guard = raw_value::guarded(true);
    let mut deserializer = Deserializer::<'de>::from_value(input.clone_ref());
    let out = T::deserialize(&mut deserializer)?;
    Ok(out)
}

impl<'deserializer> Deserializer<'deserializer> {
    pub fn from_value(input: Value) -> Self {
        Deserializer {
            input,
            option_allowed: true,
            _lifetime: PhantomData,
        }
    }

    fn deref_current(&mut self) -> Result<(), Error> {
        while let Value::Reference(_) = &self.input {
            self.input = self.input.dereference().ok_or_else(|| {
                Error::new("failed to dereference a reference while deserializing")
            })?;
        }
        Ok(())
    }

    fn sanity_check(&mut self) -> Result<(), Error> {
        if let Value::Scalar(value) = &self.input {
            match value.ty() {
                Type::Scalar(_) => Ok(()),
                Type::Other(other) => Err(Error(format!(
                    "cannot deserialize weird magic perl values ({})",
                    other
                ))),
                // These are impossible as they are all handled by different Value enum types:
                Type::Reference => Error::fail("Value::Scalar: containing a reference"),
                Type::Array => Error::fail("Value::Scalar: containing an array"),
                Type::Hash => Error::fail("Value::Scalar: containing a hash"),
            }
        } else {
            Ok(())
        }
    }

    fn get(&mut self) -> Result<&Value, Error> {
        self.deref_current()?;
        self.sanity_check()?;
        Ok(&self.input)
    }

    /// deserialize_any, preferring a string value
    fn deserialize_any_string<'de, V>(&mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.get()? {
            Value::Scalar(value) => match value.ty() {
                Type::Scalar(flags) => {
                    use crate::scalar::Flags;

                    if flags.contains(Flags::STRING) {
                        let s = unsafe { str_set_wrong_lifetime(value.pv_string_utf8()) };
                        visitor.visit_borrowed_str(s)
                    } else if flags.contains(Flags::DOUBLE) {
                        visitor.visit_f64(value.nv())
                    } else if flags.contains(Flags::INTEGER) {
                        visitor.visit_i64(value.iv() as i64)
                    } else if flags.is_empty() {
                        visitor.visit_none()
                    } else {
                        visitor.visit_unit()
                    }
                }
                _ => unreachable!(),
            },
            Value::Hash(value) => visitor.visit_map(HashAccess::new(value)),
            Value::Array(value) => visitor.visit_seq(ArrayAccess::new(value)),
            Value::Reference(_) => unreachable!(),
        }
    }

    /// deserialize_any, preferring an integer value
    fn deserialize_any_iv<'de, V>(&mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.get()? {
            Value::Scalar(value) => match value.ty() {
                Type::Scalar(flags) => {
                    use crate::scalar::Flags;

                    if flags.contains(Flags::INTEGER) {
                        visitor.visit_i64(value.iv() as i64)
                    } else if flags.contains(Flags::DOUBLE) {
                        visitor.visit_f64(value.nv())
                    } else if flags.contains(Flags::STRING) {
                        let s = unsafe { str_set_wrong_lifetime(value.pv_string_utf8()) };
                        visitor.visit_borrowed_str(s)
                    } else {
                        visitor.visit_unit()
                    }
                }
                _ => unreachable!(),
            },
            Value::Hash(value) => visitor.visit_map(HashAccess::new(value)),
            Value::Array(value) => visitor.visit_seq(ArrayAccess::new(value)),
            Value::Reference(_) => unreachable!(),
        }
    }

    /// deserialize_any, preferring a float value
    fn deserialize_any_nv<'de, V>(&mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.get()? {
            Value::Scalar(value) => match value.ty() {
                Type::Scalar(flags) => {
                    use crate::scalar::Flags;

                    if flags.contains(Flags::DOUBLE) {
                        visitor.visit_f64(value.nv())
                    } else if flags.contains(Flags::INTEGER) {
                        visitor.visit_i64(value.iv() as i64)
                    } else if flags.contains(Flags::STRING) {
                        let s = unsafe { str_set_wrong_lifetime(value.pv_string_utf8()) };
                        visitor.visit_borrowed_str(s)
                    } else {
                        visitor.visit_unit()
                    }
                }
                _ => unreachable!(),
            },
            Value::Hash(value) => visitor.visit_map(HashAccess::new(value)),
            Value::Array(value) => visitor.visit_seq(ArrayAccess::new(value)),
            Value::Reference(_) => unreachable!(),
        }
    }
}

/// We use this only for `Value`s in our deserializer. We know this works because serde says the
/// lifetime needs to only live as long as the serializer, and we feed our serializer with the data
/// from a borrowed Value (keeping references to all the contained data within perl), which lives
/// longer than the deserializer.
unsafe fn str_set_wrong_lifetime<'a, 'b>(s: &'a str) -> &'b str {
    unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(s.as_ptr(), s.len())) }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any_string(visitor)
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.get()? {
            Value::Scalar(value) => match value.ty() {
                Type::Scalar(flags) => {
                    use crate::scalar::Flags;

                    if flags.is_empty() || flags.intersects(Flags::INTEGER | Flags::DOUBLE) {
                        visitor.visit_bool(unsafe { ffi::RSPL_SvTRUE(value.sv()) })
                    } else {
                        Error::fail("expected bool value")
                    }
                }
                _ => unreachable!(),
            },
            Value::Hash(value) => visitor.visit_map(HashAccess::new(value)),
            Value::Array(value) => visitor.visit_seq(ArrayAccess::new(value)),
            Value::Reference(_) => unreachable!(),
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any_iv(visitor)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any_iv(visitor)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any_iv(visitor)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any_iv(visitor)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any_iv(visitor)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any_iv(visitor)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any_iv(visitor)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any_iv(visitor)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any_nv(visitor)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any_nv(visitor)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.get()? {
            Value::Scalar(value) => match value.ty() {
                Type::Scalar(flags) => {
                    use crate::scalar::Flags;

                    if flags.contains(Flags::INTEGER) {
                        let c = value.iv();
                        if c < 0x100 {
                            visitor.visit_char(c as u8 as char)
                        } else {
                            visitor.visit_i64(c as i64)
                        }
                    } else if flags.contains(Flags::DOUBLE) {
                        visitor.visit_f64(value.nv())
                    } else if flags.contains(Flags::STRING) {
                        let s = value.pv_string_utf8();
                        let mut chars = s.chars();
                        match chars.next() {
                            Some(ch) if chars.next().is_none() => visitor.visit_char(ch),
                            _ => {
                                let s = unsafe { str_set_wrong_lifetime(value.pv_string_utf8()) };
                                visitor.visit_borrowed_str(s)
                            }
                        }
                    } else {
                        visitor.visit_unit()
                    }
                }
                _ => unreachable!(),
            },
            Value::Hash(value) => visitor.visit_map(HashAccess::new(value)),
            Value::Array(value) => visitor.visit_seq(ArrayAccess::new(value)),
            Value::Reference(_) => unreachable!(),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.get()? {
            Value::Scalar(value) => match value.ty() {
                Type::Scalar(flags) => {
                    use crate::scalar::Flags;

                    if flags.contains(Flags::STRING) {
                        let bytes = value.pv_bytes();
                        let bytes: &'de [u8] =
                            unsafe { std::slice::from_raw_parts(bytes.as_ptr(), bytes.len()) };
                        visitor.visit_borrowed_bytes(bytes)
                    } else if flags.contains(Flags::DOUBLE) {
                        visitor.visit_f64(value.nv())
                    } else if flags.contains(Flags::INTEGER) {
                        visitor.visit_i64(value.iv() as i64)
                    } else {
                        visitor.visit_unit()
                    }
                }
                _ => unreachable!(),
            },
            Value::Hash(value) => visitor.visit_map(HashAccess::new(value)),
            Value::Array(value) => visitor.visit_seq(ArrayAccess::new(value)),
            Value::Reference(_) => unreachable!(),
        }
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        if self.option_allowed {
            if let Value::Scalar(value) = self.get()? {
                if let Type::Scalar(flags) = value.ty() {
                    if flags.is_empty() {
                        return visitor.visit_none();
                    }
                }
            }
            self.option_allowed = false;
            let res = visitor.visit_some(&mut *self);
            self.option_allowed = true;
            res
        } else {
            self.deserialize_any(visitor)
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        if name == raw_value::NAME && fields == [raw_value::VALUE] {
            if !raw_value::is_enabled() {
                return Error::fail("attempted raw value deserialization while disabled");
            }

            visitor.visit_map(RawDeserializer {
                value: Some(&self.input),
            })
        } else {
            self.deserialize_map(visitor)
        }
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let mut iter;
        // This is called for externally tagged enums only, so either a Hash with a single key, or
        // a simple string variant:
        match self.get()? {
            Value::Scalar(value) => match value.ty() {
                Type::Scalar(flags) => {
                    use crate::scalar::Flags;

                    if flags.contains(Flags::STRING) {
                        let variant = unsafe { str_set_wrong_lifetime(value.pv_string_utf8()) };
                        visitor.visit_enum(EnumDeserializer {
                            variant,
                            value: None,
                        })
                    } else {
                        Error::fail("expected an enum value")
                    }
                }
                _ => unreachable!(),
            },
            Value::Hash(hash) => {
                if hash.len() != 1 {
                    return Error::fail("expected hash with a single key");
                }

                iter = hash.shared_iter();
                let (key, value) = iter
                    .next()
                    .ok_or_else(|| Error::new("expected hash with a single key"))?;
                match std::str::from_utf8(key) {
                    Ok(variant) => {
                        // FIXME: MAKE THESE BORROWED
                        visitor.visit_enum(EnumDeserializer {
                            variant,
                            value: Some(value),
                        })
                    }
                    Err(_) => visitor.visit_enum(EnumDeserializerByteVariant {
                        variant: key,
                        value: Some(value),
                    }),
                }
            }
            _ => Error::fail("expected a string or hash for an enum"),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

struct EnumDeserializer<'a> {
    variant: &'a str,
    value: Option<Value>,
}

impl<'a, 'de> de::EnumAccess<'de> for EnumDeserializer<'a> {
    type Error = Error;
    type Variant = VariantDeserializer;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, VariantDeserializer), Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        let variant = self.variant.into_deserializer();
        let visitor = VariantDeserializer { value: self.value };
        seed.deserialize(variant).map(|v| (v, visitor))
    }
}

struct EnumDeserializerByteVariant<'a> {
    variant: &'a [u8],
    value: Option<Value>,
}

impl<'a, 'de> de::EnumAccess<'de> for EnumDeserializerByteVariant<'a> {
    type Error = Error;
    type Variant = VariantDeserializer;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, VariantDeserializer), Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        // FIXME: With serde 1.0.122 the `.to_vec()` can be dropped!
        let variant = self.variant.to_vec().into_deserializer();
        let visitor = VariantDeserializer { value: self.value };
        seed.deserialize(variant).map(|v| (v, visitor))
    }
}

struct VariantDeserializer {
    value: Option<Value>,
}

impl<'de> de::VariantAccess<'de> for VariantDeserializer {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Error> {
        match self.value {
            Some(value) => {
                de::Deserialize::deserialize(&mut Deserializer::<'de>::from_value(value))
            }
            None => Ok(()),
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        match self.value {
            Some(value) => seed.deserialize(&mut Deserializer::<'de>::from_value(value)),
            None => Error::fail("expected newtype variant, found unit variant"),
        }
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            Some(Value::Array(v)) => {
                if v.is_empty() {
                    visitor.visit_unit()
                } else {
                    visitor.visit_seq(ArrayAccess::new(&v))
                }
            }
            Some(_) => Error::fail("expected tuple variant"),
            None => Error::fail("expected tuple variant, found unit"),
        }
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            Some(Value::Hash(v)) => visitor.visit_map(HashAccess::new(&v)),
            _ => Error::fail("expected struct variant"),
        }
    }
}

/// Serde `MapAccess` intermediate type.
pub struct HashAccess<'a> {
    hash: &'a hash::Hash,
    entry: *mut ffi::HE,
    finished: bool,
    at_value: bool,
}

impl<'a> HashAccess<'a> {
    pub fn new(value: &'a hash::Hash) -> Self {
        let _ = value.shared_iter(); // reset iterator
        Self {
            hash: value,
            entry: std::ptr::null_mut(),
            finished: false,
            at_value: false,
        }
    }
}

impl<'de, 'a> MapAccess<'de> for HashAccess<'a> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Error>
    where
        K: DeserializeSeed<'de>,
    {
        if self.finished {
            return Ok(None);
        }

        if self.entry.is_null() {
            self.entry = unsafe { ffi::RSPL_hv_iternext(self.hash.hv()) };
            if self.entry.is_null() {
                self.finished = true;
                return Ok(None);
            }
        } else if self.at_value {
            return Error::fail("map access value skipped");
        }

        self.at_value = true;

        let key = unsafe { Value::from_raw_ref(ffi::RSPL_hv_iterkeysv(self.entry)) };
        seed.deserialize(&mut Deserializer::from_value(key))
            .map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Error>
    where
        V: DeserializeSeed<'de>,
    {
        if self.finished {
            return Error::fail("map access value requested after end");
        }

        if self.entry.is_null() || !self.at_value {
            return Error::fail("map access key skipped");
        }

        self.at_value = false;

        let value =
            unsafe { Value::from_raw_ref(ffi::RSPL_hv_iterval(self.hash.hv(), self.entry)) };
        self.entry = std::ptr::null_mut();

        seed.deserialize(&mut Deserializer::from_value(value))
    }
}

/// Serde `SeqAccess` intermediate type.
pub struct ArrayAccess<'a> {
    iter: array::Iter<'a>,
}

impl<'a> ArrayAccess<'a> {
    pub fn new(value: &'a array::Array) -> Self {
        Self { iter: value.iter() }
    }
}

impl<'de, 'a> SeqAccess<'de> for ArrayAccess<'a> {
    type Error = Error;

    fn next_element_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Error>
    where
        K: DeserializeSeed<'de>,
    {
        self.iter
            .next()
            .map(move |value| seed.deserialize(&mut Deserializer::from_value(value)))
            .transpose()
    }
}

struct RawDeserializer<'a> {
    value: Option<&'a Value>,
}

impl<'de, 'a> MapAccess<'de> for RawDeserializer<'a> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Error>
    where
        K: DeserializeSeed<'de>,
    {
        if self.value.is_some() {
            seed.deserialize(BorrowedStrDeserializer::new(raw_value::VALUE))
                .map(Some)
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Error>
    where
        V: DeserializeSeed<'de>,
    {
        if let Some(value) = self.value.take() {
            seed.deserialize((value.sv() as usize).into_deserializer())
        } else {
            Error::fail("map access value requested after end")
        }
    }
}
