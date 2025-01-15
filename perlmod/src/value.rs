//! The [`Value`] type is a generic perl value reference distinguishing between its types
//! automatically.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::ffi::{self, SV};
use crate::scalar::ScalarRef;
use crate::Error;
use crate::{perl_fn, raw_value};
use crate::{Array, Hash, Scalar};

/// A higher level value. This is basically an [`SV`] already cast to [`AV`](crate::ffi::AV) or
/// [`HV`](crate::ffi::HV) for arrays and hashes.
#[derive(Clone)]
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

    /// Create a new reference code reference.
    ///
    /// # Safety
    ///
    /// It is up to the user to ensure that only a valid perl XSUB is used. You should know what
    /// you're doing, as perl code WILL execute the function and expect it to behave in a
    /// particular way!
    pub unsafe fn new_xsub(xsub: perl_fn!(extern "C" fn(*mut crate::ffi::CV))) -> Self {
        unsafe {
            Self::from_raw_move(crate::ffi::RSPL_newXS_flags(
                std::ptr::null(),
                xsub as _,
                std::ptr::null(),
                std::ptr::null(),
                0,
            ) as _)
        }
    }

    /// If the value is an array, returns the associated [`Array`].
    pub fn as_array(&self) -> Option<&Array> {
        match self {
            Value::Array(v) => Some(v),
            _ => None,
        }
    }

    /// If the value is an array, returns the associated mutable [`Array`].
    pub fn as_array_mut(&mut self) -> Option<&mut Array> {
        match self {
            Value::Array(v) => Some(v),
            _ => None,
        }
    }

    /// If the value is a hash, returns the associated [`struct@Hash`].
    pub fn as_hash(&self) -> Option<&Hash> {
        match self {
            Value::Hash(v) => Some(v),
            _ => None,
        }
    }

    /// If the value is a hash, returns the associated mutable [`struct@Hash`].
    pub fn as_hash_mut(&mut self) -> Option<&mut Hash> {
        match self {
            Value::Hash(v) => Some(v),
            _ => None,
        }
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

    /// Create a new empty hash.
    pub fn new_hash() -> Self {
        Value::Hash(Hash::new())
    }

    /// Bless a reference into a package. The `Value` must be a reference.
    ///
    /// Note that a blessed value in perl can have a destructor (a `DESTROY` sub), and keeps track
    /// of references, so one can implement a class package like this:
    ///
    /// ```
    /// // 'lib' and 'file' are optional. We use 'file' here to prevent doc tests from writing out
    /// // the file.
    /// #[perlmod::package(name = "RSPM::MyThing", lib = "bless_doctest", file="/dev/null")]
    /// mod export {
    ///     # use perlmod::{Error, Value};
    ///
    ///     struct MyThing {
    ///         content: String,
    ///     }
    ///
    ///     #[export(raw_return)]
    ///     fn new(#[raw] class: Value, content: String) -> Result<Value, Error> {
    ///         let mut ptr = Box::new(MyThing { content });
    ///         // Short version:
    ///         // return Value::bless_box(class, ptr);
    ///
    ///         // Raw code:
    ///
    ///         // create a pointer value
    ///         let value = Value::new_pointer::<MyThing>(&mut *ptr);
    ///
    ///         // create a reference to it:
    ///         let value = Value::new_ref(&value);
    ///
    ///         // use the the provided class name as perl passes it along when using
    ///         // `RSPM::MyThing->new()`. Alternatively this could be hardcoded and
    ///         // `RSPM::MyThing::new()` (without an arrow) would be used instead.
    ///         let this = value.bless_sv(&class)?;
    ///
    ///         // From here on out perl will call our destructor defined below, so
    ///         // it's time to drop our reference to it!
    ///         let _perl = Box::leak(ptr);
    ///
    ///         Ok(this)
    ///     }
    ///
    ///     #[export]
    ///     fn something(#[raw] this: Value) {
    ///         let _ = this; // see the `DESTROY` sub below for how to access this.
    ///         println!("Example method callable via $foo->something()!");
    ///     }
    ///
    ///     #[export(name = "DESTROY")]
    ///     fn destroy(#[raw] this: Value) {
    ///         match this
    ///             .dereference()
    ///             .ok_or_else(|| Error::new("not a reference"))
    ///             .and_then(|this| Ok(this.pv_raw()?))
    ///         {
    ///             Ok(ptr) => {
    ///                 let value = unsafe { Box::<MyThing>::from_raw(ptr) };
    ///                 println!("Dropping value {:?}", value.content);
    ///             }
    ///             Err(err) => {
    ///                 println!("DESTROY called with invalid pointer: {}", err);
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    pub fn bless(&self, package: &str) -> Result<Value, Error> {
        let pkgsv = Scalar::new_string(package);
        self.bless_sv(&pkgsv)
    }

    /// Same as [`bless`](Self::bless()) but the package string is already a perl [`ScalarRef`].
    pub fn bless_sv(&self, pkgsv: &ScalarRef) -> Result<Value, Error> {
        let stash = unsafe { ffi::RSPL_gv_stashsv(pkgsv.sv(), 0) };
        if stash.is_null() {
            return Err(Error(format!(
                "failed to find package {:?}",
                pkgsv.pv_string_utf8()
            )));
        }

        let value = unsafe { ffi::RSPL_sv_bless(self.sv(), stash) };
        if value.is_null() {
            return Err(Error(format!(
                "failed to bless value into package {:?}",
                pkgsv.pv_string_utf8()
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
        Self::from_scalar(unsafe { Scalar::from_raw_move(ptr) })
    }

    /// Create a new reference to an existing `SV` value. This will increase the value's reference
    /// count.
    ///
    /// # Safety
    ///
    /// The caller may still need to decrease the reference count for the `ptr` source value.
    pub unsafe fn from_raw_ref(ptr: *mut SV) -> Self {
        Self::from_scalar(unsafe { Scalar::from_raw_ref(ptr) })
    }

    /// Convert a [`Scalar`] to a [`Value`].
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

    /// Turn this into a "mortal" value in perl.
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

    /// Check that the value is a reference and if so, assume it is a reference to a boxed rust
    /// type and return a reference to it.
    ///
    /// # Safety
    ///
    /// This is mainly a helper to be used for blessed values. This only checks that the value
    /// itself is any kind of reference, then assumes it contains something resembling a pointer
    /// (see [`ScalarRef::pv_raw`](ScalarRef::pv_raw())), and if so, simply casts it to `T`.
    pub unsafe fn from_ref_box<T>(&self) -> Result<&T, Error> {
        let ptr = self
            .dereference()
            .ok_or_else(|| Error::new("not a reference"))?
            .pv_raw()?;
        Ok(unsafe { &*(ptr as *const T) })
    }

    /// Check that the value is a reference and blessed into a particular package name. If so,
    /// assume it is a referenced to a boxed rust type and return a reference to it.
    ///
    /// # Safety
    ///
    /// See [`Value::from_ref_box`]. This additionally uses [`reftype`](ScalarRef::reftype) to
    /// check that the passed value was indeed blessed into the provided `package` name. Other than
    /// that, it cannot verify the the contained pointer is truly a `T`.
    pub unsafe fn from_blessed_box<'a, T>(&'a self, package: &'_ str) -> Result<&'a T, Error> {
        let ptr = self
            .dereference()
            .ok_or_else(|| Error::new("not a reference"))?;

        let reftype = ptr.reftype(true);
        if reftype != package {
            return Err(Error::new_owned(format!(
                "value not blessed into {package:?} (`ref` returned {reftype:?})",
            )));
        }

        Ok(unsafe { &*(ptr.pv_raw()? as *const T) })
    }

    /// Take ownership of a boxed value and create a perl value blessed into a package name.
    ///
    /// Note that this leaks the box. To let perl properly drop the value, the class name (which
    /// must be a valid perl class name), must include a `DESTROY` sub implementing a compatible
    /// destructor. Like so:
    ///
    /// ```
    /// #[perlmod::package(name = "My::Thing")]
    /// mod export {
    ///     use std::sync::Mutex;
    ///
    ///     use perlmod::{Error, Value};
    ///
    ///     const CLASSNAME: &str = "My::Thing";
    ///
    ///     /// Some thing.
    ///     pub struct Thing {
    ///         stuff: String,
    ///     }
    ///
    ///     /// Constructor for a Thing.
    ///     /// Prior to perlmod 0.6, the `raw_return` and `#[raw]` attributes were necessary, now
    ///     /// they're optional but produce slightly less code.
    ///     #[export(raw_return)]
    ///     pub fn new(#[raw] class: Value, stuff: String) -> Result<Value, Error> {
    ///         Value::bless_box(class, Box::new(Thing { stuff }))
    ///     }
    ///
    ///     /// Destructor for a Thing.
    ///     #[export(name = "DESTROY")]
    ///     fn destroy(#[raw] this: Value) {
    ///         perlmod::destructor!(this, Thing: CLASSNAME);
    ///     }
    ///
    ///     /// Convenience helper for generating methods more quickly.
    ///     impl<'a> TryFrom<&'a Value> for &'a Thing {
    ///         type Error = Error;
    ///
    ///         fn try_from(value: &'a Value) -> Result<&'a Thing, Error> {
    ///             Ok(unsafe { value.from_blessed_box(CLASSNAME)? })
    ///         }
    ///     }
    ///
    ///     /// The `try_from_ref` attribute tells perlmod to use `TryFrom::try_from(&value)` to
    ///     /// pass to `say_hello`. The conversion will `die` in perl on failure.
    ///     #[export]
    ///     pub fn say_hello(#[try_from_ref] this: &Thing) {
    ///         println!("Hello, {}", this.stuff);
    ///     }
    /// }
    /// ```
    pub fn bless_box<T>(class: Value, mut box_: Box<T>) -> Result<Value, Error> {
        let value = Value::new_pointer::<T>(&mut *box_);
        let value = Value::new_ref(&value);
        let this = value.bless_sv(&class)?;
        let _perl = Box::leak(box_);
        Ok(this)
    }

    /// Attempt to create a substring, provided the contained value is actually a string.
    pub fn substr<I>(&self, index: I) -> Result<Value, Error>
    where
        I: std::slice::SliceIndex<[u8], Output = [u8]>,
    {
        match self {
            Value::Scalar(s) => s.substr(index).map(Value::Scalar),
            _ => Err(Error::new("substr called on non-scalar")),
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

impl From<raw_value::RawValue> for Value {
    fn from(value: raw_value::RawValue) -> Self {
        value.into_inner()
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
            Value::Scalar(v) => v,
            Value::Reference(v) => v,
            Value::Array(v) => v,
            Value::Hash(v) => v,
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

impl AsRef<ScalarRef> for Value {
    #[inline]
    fn as_ref(&self) -> &ScalarRef {
        self
    }
}

impl AsMut<ScalarRef> for Value {
    #[inline]
    fn as_mut(&mut self) -> &mut ScalarRef {
        &mut *self
    }
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::Error;

        if raw_value::is_enabled() {
            raw_value::serialize_raw(self, serializer)
        } else {
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
                Ok(Value::new_int(isize::from(value)))
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

                impl Visitor<'_> for KeyClassifier {
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

        if raw_value::is_enabled() {
            raw_value::RawValue::deserialize(deserializer).map(Value::from)
        } else {
            deserializer.deserialize_any(ValueVisitor)
        }
    }
}
