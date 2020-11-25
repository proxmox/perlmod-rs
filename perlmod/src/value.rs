//! The [`Value`] type is a generic perl value reference distinguishing between its types
//! automatically.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::ffi::{self, SV};
use crate::scalar::ScalarRef;
use crate::Error;
use crate::{Array, Hash, Scalar};

/// A higher level value. This is basically an `SV` already cast to `AV` or `HV` for arrays and
/// hashes.
pub enum Value {
    Scalar(Scalar),
    Reference(Scalar),
    Array(Array),
    Hash(Hash),
}

impl Value {
    /// Create a new undef value:
    pub fn new_undef() -> Self {
        Value::Scalar(Scalar::new_undef())
    }

    /// Create a new integer value:
    pub fn new_int(v: isize) -> Self {
        Value::Scalar(Scalar::new_int(v))
    }

    /// Create a new unsigned integer value:
    pub fn new_uint(v: usize) -> Self {
        Value::Scalar(Scalar::new_uint(v))
    }

    /// Create a new floating point value.
    pub fn new_float(v: f64) -> Self {
        Value::Scalar(Scalar::new_float(v))
    }

    /// Create a new string value.
    pub fn new_string(s: &str) -> Self {
        Value::Scalar(Scalar::new_string(s))
    }

    /// Create a new byte string.
    pub fn new_bytes(s: &[u8]) -> Self {
        Value::Scalar(Scalar::new_bytes(s))
    }

    /// Convenience method to create a new raw pointer value. Note that pointers are stored as
    /// arbitrary "byte strings" and any such byte string value can be interpreted as a raw pointer.
    pub fn new_pointer<T>(s: *mut T) -> Self {
        Self::new_bytes(&(s as usize).to_ne_bytes())
    }

    /// Create an actual perl reference to the value. (The equivalent of perl's backslash
    /// operator).
    pub fn new_ref<T>(value: &T) -> Self
    where
        T: std::ops::Deref<Target = ScalarRef>,
    {
        Value::Reference(unsafe { Scalar::from_raw_move(ffi::RSPL_newRV_inc(value.sv())) })
    }

    /// Bless a reference into a package. The `Value` must be a reference.
    pub fn bless(&self, package: &str) -> Result<Value, Error> {
        let pkgsv = Scalar::new_string(package);
        let stash = unsafe { ffi::RSPL_gv_stashsv(pkgsv.sv(), 0) };
        if stash.is_null() {
            return Err(Error(format!("failed to find package {:?}", package)));
        }

        let value = unsafe { ffi::RSPL_sv_bless(self.sv(), stash) };
        if value.is_null() {
            return Err(Error(format!(
                "failed to bless value into package {:?}",
                package
            )));
        }

        Ok(Value::Reference(unsafe { Scalar::from_raw_ref(value) }))
    }

    /// Take over a raw `SV` value, assuming that we then own a reference to it.
    ///
    /// # Safety
    ///
    /// This does not change the value's reference count, it is assumed that we're taking ownership
    /// of one reference.
    ///
    /// The caller must ensure that it is safe to decrease the reference count later on, or use
    /// `into_raw()` instead of letting the `Value` get dropped.
    pub unsafe fn from_raw_move(ptr: *mut SV) -> Self {
        Self::from_scalar(Scalar::from_raw_move(ptr as *mut SV))
    }

    /// Create a new reference to an existing `SV` value. This will increase the value's reference
    /// count.
    ///
    /// # Safety
    ///
    /// The caller may still need to decrease the reference count for the `ptr` source value.
    pub unsafe fn from_raw_ref(ptr: *mut SV) -> Self {
        Self::from_scalar(Scalar::from_raw_ref(ptr as *mut SV))
    }

    pub fn from_scalar(scalar: Scalar) -> Self {
        Self::from(scalar)
    }

    /// Create a new reference to this value.
    pub fn clone_ref(&self) -> Self {
        match self {
            Value::Scalar(v) => Value::Scalar(v.clone_ref()),
            Value::Reference(v) => Value::Reference(v.clone_ref()),
            Value::Array(v) => Value::Array(v.clone_ref()),
            Value::Hash(v) => Value::Hash(v.clone_ref()),
        }
    }

    /// Dereference this reference value.
    pub fn dereference(&self) -> Option<Value> {
        match self {
            Value::Reference(v) => v.dereference().map(Value::from_scalar),
            _ => None,
        }
    }

    /// Turn this into a raw `SV` transferring control of one reference count.
    pub fn into_raw(self) -> *mut SV {
        match self {
            Value::Scalar(v) => v.into_raw(),
            Value::Reference(v) => v.into_raw(),
            Value::Array(v) => v.into_scalar().into_raw(),
            Value::Hash(v) => v.into_scalar().into_raw(),
        }
    }

    pub fn into_mortal(self) -> crate::scalar::Mortal {
        match self {
            Value::Scalar(v) => v.into_mortal(),
            Value::Reference(v) => v.into_mortal(),
            Value::Array(v) => v.into_scalar().into_mortal(),
            Value::Hash(v) => v.into_scalar().into_mortal(),
        }
    }

    /// If this is value is an array, get the value at the specified index.
    pub fn get(&self, index: usize) -> Option<Value> {
        if let Value::Array(a) = self {
            a.get(index)
        } else {
            None
        }
    }
}

impl From<Scalar> for Value {
    fn from(scalar: Scalar) -> Self {
        unsafe {
            if ffi::RSPL_is_array(scalar.sv()) {
                Value::Array(Array::from_scalar(scalar))
            } else if ffi::RSPL_is_hash(scalar.sv()) {
                Value::Hash(Hash::from_scalar(scalar))
            } else if ffi::RSPL_is_reference(scalar.sv()) {
                Value::Reference(scalar)
            } else {
                Value::Scalar(scalar)
            }
        }
    }
}

impl From<Hash> for Value {
    fn from(hash: Hash) -> Self {
        Value::Hash(hash)
    }
}

impl From<Array> for Value {
    fn from(array: Array) -> Self {
        Value::Array(array)
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use fmt::Debug;
        match self {
            Value::Scalar(v) => Debug::fmt(v, f),
            Value::Reference(v) => Debug::fmt(v, f),
            Value::Array(v) => Debug::fmt(v, f),
            Value::Hash(v) => Debug::fmt(v, f),
        }
    }
}

impl core::ops::Deref for Value {
    type Target = ScalarRef;

    fn deref(&self) -> &Self::Target {
        match self {
            Value::Scalar(v) => &*v,
            Value::Reference(v) => &*v,
            Value::Array(v) => &*v,
            Value::Hash(v) => &*v,
        }
    }
}

impl core::ops::DerefMut for Value {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Value::Scalar(v) => &mut *v,
            Value::Reference(v) => &mut *v,
            Value::Array(v) => &mut *v,
            Value::Hash(v) => &mut *v,
        }
    }
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::Error;

        match self {
            Value::Scalar(this) => this.serialize(serializer),
            Value::Reference(this) => Value::from(
                this.dereference()
                    .ok_or_else(|| S::Error::custom("failed to dereference perl value"))?,
            )
            .serialize(serializer),
            Value::Array(value) => value.serialize(serializer),
            Value::Hash(value) => value.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Visitor;

        struct ValueVisitor;

        impl<'de> Visitor<'de> for ValueVisitor {
            type Value = Value;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("any valid PERL value")
            }

            #[inline]
            fn visit_bool<E>(self, value: bool) -> Result<Value, E> {
                Ok(Value::new_int(if value { 1 } else { 0 }))
            }

            #[inline]
            fn visit_i64<E>(self, value: i64) -> Result<Value, E> {
                Ok(Value::new_int(value as isize))
            }

            #[inline]
            fn visit_u64<E>(self, value: u64) -> Result<Value, E> {
                Ok(Value::new_uint(value as usize))
            }

            #[inline]
            fn visit_f64<E>(self, value: f64) -> Result<Value, E> {
                Ok(Value::new_float(value))
            }

            #[inline]
            fn visit_str<E>(self, value: &str) -> Result<Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::new_string(value))
            }

            #[inline]
            fn visit_string<E>(self, value: String) -> Result<Value, E>
            where
                E: serde::de::Error,
            {
                self.visit_str(&value)
            }

            #[inline]
            fn visit_none<E>(self) -> Result<Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::new_undef())
            }

            #[inline]
            fn visit_some<D>(self, deserializer: D) -> Result<Value, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                Deserialize::deserialize(deserializer)
            }

            #[inline]
            fn visit_unit<E>(self) -> Result<Value, E> {
                Ok(Value::new_undef())
            }

            #[inline]
            fn visit_seq<V>(self, mut visitor: V) -> Result<Value, V::Error>
            where
                V: serde::de::SeqAccess<'de>,
            {
                let array = Array::new();

                while let Some(elem) = visitor.next_element()? {
                    array.push(elem);
                }

                Ok(Value::Array(array))
            }

            fn visit_map<V>(self, mut visitor: V) -> Result<Value, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                // We use this to hint the deserializer that we're expecting a string-ish value.
                struct KeyClassifier;
                struct KeyClass(String);

                impl<'de> serde::de::DeserializeSeed<'de> for KeyClassifier {
                    type Value = KeyClass;

                    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
                    where
                        D: serde::Deserializer<'de>,
                    {
                        deserializer.deserialize_str(self)
                    }
                }

                impl<'de> Visitor<'de> for KeyClassifier {
                    type Value = KeyClass;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("a string key")
                    }

                    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
                    where
                        E: serde::de::Error,
                    {
                        Ok(KeyClass(s.to_owned()))
                    }

                    fn visit_string<E>(self, s: String) -> Result<Self::Value, E>
                    where
                        E: serde::de::Error,
                    {
                        Ok(KeyClass(s))
                    }
                }

                let hash = Hash::new();
                while let Some(key) = visitor.next_key_seed(KeyClassifier)? {
                    let value: Value = visitor.next_value()?;
                    hash.insert(&key.0, value);
                }
                Ok(Value::from(hash))
            }
        }

        deserializer.deserialize_any(ValueVisitor)
    }
}
