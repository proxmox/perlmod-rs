//! Module dealing with perl [`Hash`](crate::Hash)es. ([`HV`](crate::ffi::HV) pointers).

use std::convert::TryFrom;

use crate::error::CastError;
use crate::ffi::{self, HV, SV};
use crate::raw_value;
use crate::scalar::{Scalar, ScalarRef};
use crate::Value;

/// An owned reference to a perl hash value (HV).
///
/// This keeps a reference to a value which lives in the perl interpreter.
#[derive(Clone)]
#[repr(transparent)]
pub struct Hash(Scalar);

#[allow(clippy::new_without_default)]
impl Hash {
    /// Create a new array value.
    pub fn new() -> Self {
        unsafe { Self::from_raw_move(ffi::RSPL_newHV()) }
    }

    /// Turn this into a `Scalar`. The underlying perl value does not change, this is a pure type
    /// cast down to a less specific "pointer" type.
    pub fn into_scalar(self) -> Scalar {
        self.0
    }

    /// Get the internal perl value as a low-level `HV` pointer.
    pub fn hv(&self) -> *mut HV {
        self.0.sv() as *mut HV
    }

    /// "Downcast" a `Scalar` into a `Hash`. The caller must verify that this is legal.
    ///
    /// # Safety
    ///
    /// The caller must verify that this is legal.
    pub unsafe fn from_scalar(scalar: Scalar) -> Self {
        Self(scalar)
    }

    /// Take over a raw `HV` value, assuming that we then own a reference to it.
    ///
    /// # Safety
    ///
    /// This does not change the value's reference count, it is assumed that we're taking ownership
    /// of one reference.
    ///
    /// The caller must ensure that it is safe to decrease the reference count later on, or use
    /// [`into_raw()`](Value::into_raw()) instead of letting the [`Hash`](struct@Hash) get dropped.
    pub unsafe fn from_raw_move(ptr: *mut HV) -> Self {
        Self(Scalar::from_raw_move(ptr as *mut SV))
    }

    /// Create a new reference to an existing [`HV`] value. This will increase the value's
    /// reference count.
    ///
    /// # Safety
    ///
    /// The caller may still need to decrease the reference count for the `ptr` source value.
    pub unsafe fn from_raw_ref(ptr: *mut HV) -> Self {
        Self(Scalar::from_raw_ref(ptr as *mut SV))
    }

    /// Create a new reference to this value.
    pub fn clone_ref(&self) -> Self {
        Self(self.0.clone_ref())
    }

    /// Get the number of keys in this hash.
    pub fn len(&self) -> usize {
        unsafe { ffi::RSPL_HvTOTALKEYS(self.hv()) }
    }

    /// Check if this is an empty hash.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get a value from the hash. Note that this only uses utf8 strings. For a more generic method
    /// see `get_by_bytes`.
    pub fn get(&self, key: &str) -> Option<Value> {
        self.get_by_bytes(key.as_bytes())
    }

    /// Get a value from the hash, but with a raw byte string as index.
    pub fn get_by_bytes(&self, key: &[u8]) -> Option<Value> {
        let sv: *mut *mut SV = unsafe {
            ffi::RSPL_hv_fetch(
                self.hv(),
                key.as_ptr() as *const libc::c_char,
                key.len() as i32,
                0,
            )
        };
        if sv.is_null() {
            None
        } else {
            Some(unsafe { Value::from_raw_ref(*sv) })
        }
    }

    /// Insert a value into the hash.
    pub fn insert(&self, key: &str, value: Value) {
        self.insert_by_bytes(key.as_bytes(), value);
    }

    /// Insert a value into the hash with a byte string as key.
    pub fn insert_by_bytes(&self, key: &[u8], value: Value) {
        unsafe {
            ffi::RSPL_hv_store(
                self.hv(),
                key.as_ptr() as *const u8 as *const libc::c_char,
                key.len() as i32,
                value.into_raw(),
            );
        }
    }

    /// Insert a value using an existin value as a key.
    pub fn insert_by_value(&self, key: &Value, value: Value) {
        unsafe {
            ffi::RSPL_hv_store_ent(self.hv(), key.sv(), value.into_raw());
        }
    }

    /// Get the *shared* iterator over this hash's elements.
    ///
    /// Note that this uses the hash's internal iterator, so any other iterator as well as `each`
    /// statement within perl code is affected by it, and it is usually a bad idea to have multiple
    /// iterators over the same hash simultaneously.
    pub fn shared_iter(&self) -> Iter {
        unsafe {
            ffi::RSPL_hv_iterinit(self.hv());
        }
        Iter { hash: self }
    }
}

impl core::ops::Deref for Hash {
    type Target = ScalarRef;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl core::ops::DerefMut for Hash {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}

impl TryFrom<Scalar> for Hash {
    type Error = CastError;

    fn try_from(scalar: Scalar) -> Result<Self, CastError> {
        if unsafe { ffi::RSPL_is_hash(scalar.sv()) } {
            Ok(Self(scalar))
        } else {
            Err(CastError)
        }
    }
}

impl std::fmt::Debug for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{{HASH}}")
    }
}

/// An iterator over a perl array.
///
/// Perl hashes have an integrated iterator. Perl goes to great lengths to make it impossible to
/// properly iterate over a hash without messing with the hash's internal state, so contrary to the
/// array iterator, this iterator always references an existing [`Hash`](crate::Hash).
pub struct Iter<'a> {
    hash: &'a Hash,
}

impl<'a> Iterator for Iter<'a> {
    type Item = (&'a [u8], Value);

    fn next(&mut self) -> Option<Self::Item> {
        let mut key: *mut libc::c_char = std::ptr::null_mut();
        let mut keylen: i32 = 0;
        let value = unsafe { ffi::RSPL_hv_iternextsv(self.hash.hv(), &mut key, &mut keylen) };
        if value.is_null() {
            return None;
        }

        unsafe {
            Some((
                std::slice::from_raw_parts(key as *mut u8, keylen as usize),
                Value::from_raw_ref(value),
            ))
        }
    }
}

impl serde::Serialize for Hash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        if raw_value::is_enabled() {
            return raw_value::serialize_raw(&self, serializer);
        }

        let mut map = serializer.serialize_map(Some(self.len()))?;
        for (k, v) in self.shared_iter() {
            map.serialize_key(&k)?;
            map.serialize_value(&v)?;
        }
        map.end()
    }
}
