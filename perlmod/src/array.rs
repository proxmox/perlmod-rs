//! Module dealing with perl [`Array`](crate::Array)s. ([`AV`](crate::ffi::AV) pointers).

use std::convert::TryFrom;
use std::marker::PhantomData;

use crate::error::CastError;
use crate::ffi::{self, AV, SV};
use crate::raw_value;
use crate::scalar::{Scalar, ScalarRef};
use crate::Value;

/// An owned reference to a perl array value (AV).
///
/// This keeps a reference to a value which lives in the perl interpreter.
#[derive(Clone)]
#[repr(transparent)]
pub struct Array(Scalar);

#[allow(clippy::new_without_default)]
impl Array {
    /// Create a new array value.
    pub fn new() -> Self {
        unsafe { Self::from_raw_move(ffi::RSPL_newAV()) }
    }

    /// Turn this into a [`Scalar`]. The underlying perl value does not change, this is a pure type
    /// cast down to a less specific "pointer" type.
    pub fn into_scalar(self) -> Scalar {
        self.0
    }

    /// Get the internal perl value as a low-level [`AV`] pointer.
    pub fn av(&self) -> *mut AV {
        self.0.sv() as *mut AV
    }

    /// "Downcast" a [`Scalar`] into an [`Array`].
    ///
    /// # Safety
    ///
    /// The caller must verify that this is legal.
    pub unsafe fn from_scalar(scalar: Scalar) -> Self {
        Self(scalar)
    }

    /// Take over a raw [`AV`] value, assuming that we then own a reference to it.
    ///
    /// # Safety
    ///
    /// This does not change the value's reference count, it is assumed that we're taking ownership
    /// of one reference.
    ///
    /// The caller must ensure that it is safe to decrease the reference count later on, or use
    /// [`into_raw()`](Value::into_raw()) instead of letting the [`Array`] get dropped.
    pub unsafe fn from_raw_move(ptr: *mut AV) -> Self {
        Self(Scalar::from_raw_move(ptr as *mut SV))
    }

    /// Create a new reference to an existing [`AV`] value. This will increase the value's
    /// reference count.
    ///
    /// # Safety
    ///
    /// The caller may still need to decrease the reference count for the `ptr` source value.
    pub unsafe fn from_raw_ref(ptr: *mut AV) -> Self {
        Self(Scalar::from_raw_ref(ptr as *mut SV))
    }

    /// Create a new reference to this value.
    pub fn clone_ref(&self) -> Self {
        Self(self.0.clone_ref())
    }

    /// Get the length of the array.
    pub fn len(&self) -> usize {
        // perl returns the highest index, not the length!
        unsafe { ffi::RSPL_av_len(self.av()).wrapping_add(1) }
    }

    /// Check if this is an empty array.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get a value from the array.
    pub fn get(&self, index: usize) -> Option<Value> {
        let index = index as libc::ssize_t;
        let sv: *mut *mut SV = unsafe { ffi::RSPL_av_fetch(self.av(), index, 0) };
        if sv.is_null() {
            None
        } else {
            Some(unsafe { Value::from_raw_ref(*sv) })
        }
    }

    /// Create an iterator over this array's values.
    pub fn iter(&self) -> Iter {
        Iter {
            array: self.clone_ref(),
            at: 0,
            _phantom: PhantomData,
        }
    }

    /// Pre-extend the array to up to the specified length..
    pub fn reserve(&self, more: usize) {
        if more == 0 {
            return;
        }
        let idx = self.len() + more - 1;
        unsafe {
            ffi::RSPL_av_extend(self.av(), idx as libc::ssize_t);
        }
    }

    /// Push a value onto the array.
    pub fn push(&self, value: Value) {
        unsafe {
            ffi::RSPL_av_push(self.av(), value.into_raw());
        }
    }

    /// Pop a value off of the array's end.
    pub fn pop(&self) -> Option<Value> {
        if self.is_empty() {
            None
        } else {
            Some(unsafe { Value::from_raw_move(ffi::RSPL_av_pop(self.av())) })
        }
    }
}

impl core::ops::Deref for Array {
    type Target = ScalarRef;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl core::ops::DerefMut for Array {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}

impl TryFrom<Scalar> for Array {
    type Error = CastError;

    fn try_from(scalar: Scalar) -> Result<Self, CastError> {
        if unsafe { ffi::RSPL_is_array(scalar.sv()) } {
            Ok(Self(scalar))
        } else {
            Err(CastError)
        }
    }
}

impl std::fmt::Debug for Array {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "[")?;
        let mut comma = false;
        for i in self {
            if comma {
                write!(f, ", {:?}", i)?;
            } else {
                comma = true;
                write!(f, "{:?}", i)?;
            }
        }
        write!(f, "]")?;
        Ok(())
    }
}

/// An iterator over a perl array.
///
/// Technically the iterator always holds a reference count on the [`AV`] pointer, but we still
/// distinguish between an iterator going over a borrowed [`Array`] and one coming from
/// [`IntoIterator`](std::iter::IntoIterator).
pub struct Iter<'a> {
    array: Array,
    at: usize,
    _phantom: PhantomData<&'a Array>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        let at = self.at;
        if at < self.array.len() {
            self.at += 1;
            self.array.get(at)
        } else {
            None
        }
    }
}

impl IntoIterator for Array {
    type Item = Value;
    type IntoIter = Iter<'static>;

    fn into_iter(self) -> Self::IntoIter {
        Iter {
            array: self,
            at: 0,
            _phantom: PhantomData,
        }
    }
}

impl<'a> IntoIterator for &'a Array {
    type Item = Value;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl serde::Serialize for Array {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeSeq;

        if raw_value::is_enabled() {
            return raw_value::serialize_raw(&self, serializer);
        }

        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for i in self {
            seq.serialize_element(&i)?;
        }
        seq.end()
    }
}
