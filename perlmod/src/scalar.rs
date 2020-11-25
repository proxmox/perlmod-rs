//! Module containing the [`Scalar`] and [`Mortal`] types.

use std::convert::TryInto;
use std::mem;

use bitflags::bitflags;

use crate::ffi::{self, SV};
use crate::Error;
use crate::Value;

/// An owned reference to a perl value.
///
/// This keeps a reference to a value which lives in the perl interpreter.
/// This derefs to a `ScalarRef` which implements most of the basic functionality common to all
/// `SV` related types.
#[repr(transparent)]
pub struct Scalar(*mut SV);

impl Scalar {
    /// Turn this into a "mortal" value. This will move this value's owned reference onto the
    /// mortal stack to be cleaned up after the next perl statement if no more references exist.
    ///
    /// (To be garbage collected after this perl-statement.)
    pub fn into_mortal(self) -> Mortal {
        Mortal(unsafe { ffi::RSPL_sv_2mortal(self.into_raw()) })
    }

    /// Turn this into a raw `SV` transferring control of one reference count.
    pub fn into_raw(self) -> *mut SV {
        let ptr = self.0;
        core::mem::forget(self);
        ptr
    }

    /// Create a wrapping `Scalar` from an `SV` pointer. The `Scalar` takes over the owned
    /// reference from the passed `SV`, which means it must not be a mortal reference.
    ///
    /// # Safety
    ///
    /// This does not change the value's reference count, it is assumed that we're taking ownership
    /// of one reference.
    ///
    /// The caller must ensure that it is safe to decrease the reference count later on, or use
    /// `into_raw()` instead of letting the `Scalar` get dropped.
    pub unsafe fn from_raw_move(ptr: *mut SV) -> Self {
        Self(ptr)
    }

    /// Increase the reference count on an `SV` pointer.
    ///
    /// # Safety
    ///
    /// The caller may still need to decrease the reference count for the `ptr` source value.
    pub unsafe fn from_raw_ref(ptr: *mut SV) -> Self {
        Self::from_raw_move(ffi::RSPL_SvREFCNT_inc(ptr))
    }

    /// Create a reference to `PL_sv_undef`.
    pub fn new_undef() -> Self {
        unsafe { Self::from_raw_ref(ffi::RSPL_get_undef()) }
    }

    /// Create a reference to `PL_sv_yes`.
    pub fn new_yes() -> Self {
        unsafe { Self::from_raw_ref(ffi::RSPL_get_yes()) }
    }

    /// Create a reference to `PL_sv_no`.
    pub fn new_no() -> Self {
        unsafe { Self::from_raw_ref(ffi::RSPL_get_no()) }
    }

    /// Create a new integer value:
    pub fn new_int(v: isize) -> Self {
        unsafe { Self::from_raw_move(ffi::RSPL_newSViv(v)) }
    }

    /// Create a new unsigned integer value:
    pub fn new_uint(v: usize) -> Self {
        unsafe { Self::from_raw_move(ffi::RSPL_newSVuv(v)) }
    }

    /// Create a new floating point value.
    pub fn new_float(v: f64) -> Self {
        unsafe { Self::from_raw_move(ffi::RSPL_newSVnv(v)) }
    }

    /// Create a new string value.
    pub fn new_string(s: &str) -> Self {
        Self::new_bytes(s.as_bytes())
    }

    /// Create a new byte string.
    pub fn new_bytes(s: &[u8]) -> Self {
        unsafe {
            Self::from_raw_move(ffi::RSPL_newSVpvn(
                s.as_ptr() as *const libc::c_char,
                s.len() as libc::size_t,
            ))
        }
    }

    /// Convenience method to create a new raw pointer value. Note that pointers are stored as
    /// arbitrary "byte strings" and any such byte string value can be interpreted as a raw pointer.
    pub fn new_pointer<T>(s: *mut T) -> Self {
        Self::new_bytes(&(s as usize).to_ne_bytes())
    }
}

impl Drop for Scalar {
    fn drop(&mut self) {
        unsafe {
            ffi::RSPL_SvREFCNT_dec(self.sv());
        }
    }
}

impl core::ops::Deref for Scalar {
    type Target = ScalarRef;

    fn deref(&self) -> &Self::Target {
        unsafe { &*(self.0 as *mut ScalarRef) }
    }
}

impl core::ops::DerefMut for Scalar {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(self.0 as *mut ScalarRef) }
    }
}

/// A value which has been pushed to perl's "mortal stack".
#[repr(transparent)]
pub struct Mortal(*mut SV);

impl Mortal {
    /// Get the inner value.
    pub fn into_raw(self) -> *mut SV {
        self.0
    }
}

impl core::ops::Deref for Mortal {
    type Target = ScalarRef;

    fn deref(&self) -> &Self::Target {
        unsafe { &*(self.0 as *mut ScalarRef) }
    }
}

impl core::ops::DerefMut for Mortal {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(self.0 as *mut ScalarRef) }
    }
}

pub struct ScalarRef;

bitflags! {
    /// Represents the types a `Value` can contain. Values can usually contain multiple scalar types
    /// at once and it is unclear which is the "true" type, so we can only check whether a value
    /// contains something, not what it is originally meant to be!
    ///
    /// NOTE: The values must be the same as in our c glue code!
    pub struct Flags: u8 {
        const INTEGER = 1;
        const DOUBLE = 2;
        const STRING = 4;
    }
}

/// While scalar types aren't clearly different from another, complex types are, so we do
/// distinguish between these:
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Type {
    Scalar(Flags),
    Reference,
    Array,
    Hash,
    Other(u8),
}

impl ScalarRef {
    pub(crate) fn sv(&self) -> *mut SV {
        self as *const ScalarRef as *const SV as *mut SV
    }

    /// Get some information about the value's type.
    pub fn ty(&self) -> Type {
        unsafe {
            if ffi::RSPL_is_reference(self.sv()) {
                Type::Reference
            } else {
                let flags = ffi::RSPL_type_flags(self.sv());
                if flags > 0xff {
                    panic!("bad C type returned");
                } else if flags != 0 {
                    Type::Scalar(Flags::from_bits(flags as u8).unwrap())
                } else if ffi::RSPL_is_array(self.sv()) {
                    Type::Array
                } else if ffi::RSPL_is_hash(self.sv()) {
                    Type::Hash
                } else {
                    Type::Other(ffi::RSPL_svtype(self.sv()) as u8)
                }
            }
        }
    }

    /// Dereference this reference.
    pub fn dereference(&self) -> Option<Scalar> {
        let ptr = unsafe { ffi::RSPL_dereference(self.sv()) };
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { Scalar::from_raw_ref(ptr) })
        }
    }

    /// Coerce to a double value. (perlxs SvNV).
    pub fn nv(&self) -> f64 {
        unsafe { ffi::RSPL_SvNV(self.sv()) }
    }

    /// Coerce to an integer value. (perlxs SvIV).
    pub fn iv(&self) -> isize {
        unsafe { ffi::RSPL_SvIV(self.sv()) }
    }

    /// Coerce to an utf8 string value. (perlxs SvPVutf8)
    pub fn pv_utf8(&self) -> &str {
        unsafe {
            let mut len: libc::size_t = 0;
            let ptr = ffi::RSPL_SvPVutf8(self.sv(), &mut len) as *const u8;
            std::str::from_utf8_unchecked(std::slice::from_raw_parts(ptr, len))
        }
    }

    /// Coerce to a string without utf8 encoding. (perlxs SvPV)
    pub fn pv_string_bytes(&self) -> &[u8] {
        unsafe {
            let mut len: libc::size_t = 0;
            let ptr = ffi::RSPL_SvPV(self.sv(), &mut len) as *const u8;
            std::slice::from_raw_parts(ptr, len)
        }
    }

    /// Coerce to a byte-string. (perlxs SvPVbyte)
    pub fn pv_bytes(&self) -> &[u8] {
        unsafe {
            let mut len: libc::size_t = 0;
            let ptr = ffi::RSPL_SvPVbyte(self.sv(), &mut len) as *const u8;
            std::slice::from_raw_parts(ptr, len)
        }
    }

    /// Interpret the byte string as a raw pointer.
    pub fn pv_raw<T>(&self) -> Result<*mut T, Error> {
        let bytes = self.pv_bytes();

        let bytes: [u8; mem::size_of::<usize>()] = bytes
            .try_into()
            .map_err(|err| Error(format!("invalid value for pointer: {}", err)))?;

        Ok(usize::from_ne_bytes(bytes) as *mut T)
    }

    /// Interpret the byte string as a pointer and return it as a reference for convenience.
    ///
    /// # Safety
    ///
    /// The user is responsible for making sure the underlying pointer is correct.
    pub unsafe fn pv_ref<T>(&self) -> Result<&T, Error> {
        self.pv_raw().map(|p| unsafe { &*p })
    }

    /// Interpret the byte string as a pointer and return it as a mutable reference for
    /// convenience.
    ///
    /// # Safety
    ///
    /// The user is responsible for making sure the underlying pointer is correct.
    pub unsafe fn pv_mut_ref<T>(&self) -> Result<&mut T, Error> {
        self.pv_raw().map(|p| unsafe { &mut *p })
    }

    /// Create another owned reference to this value.
    pub fn clone_ref(&self) -> Scalar {
        unsafe { Scalar::from_raw_ref(self.sv()) }
    }

    /// Convenience check for SVt_NULL
    pub fn is_undef(&self) -> bool {
        0 == unsafe { ffi::RSPL_type_flags(self.sv()) }
    }

    /// Turn this into a `Value`.
    pub fn into_value(self) -> Value {
        Value::from_scalar(self.clone_ref())
    }
}

impl std::fmt::Debug for Scalar {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let this: &ScalarRef = &self;
        std::fmt::Debug::fmt(this, f)
    }
}

impl std::fmt::Debug for ScalarRef {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use std::fmt::Debug;
        match self.ty() {
            Type::Scalar(flags) => {
                if flags.intersects(Flags::STRING) {
                    Debug::fmt(self.pv_utf8(), f)
                } else if flags.intersects(Flags::INTEGER) {
                    write!(f, "{}", self.iv())
                } else if flags.intersects(Flags::DOUBLE) {
                    write!(f, "{}", self.nv())
                } else {
                    write!(f, "<unhandled scalar>")
                }
            }
            Type::Reference => write!(f, "<*REFERENCE>"),
            Type::Array => write!(f, "<*ARRAY>"),
            Type::Hash => write!(f, "<*HASH>"),
            Type::Other(_) => write!(f, "<*PERLTYPE>"),
        }
    }
}

impl serde::Serialize for Scalar {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::Error;

        match self.ty() {
            Type::Scalar(flags) => {
                if flags.contains(Flags::STRING) {
                    serializer.serialize_str(self.pv_utf8())
                } else if flags.contains(Flags::DOUBLE) {
                    serializer.serialize_f64(self.nv())
                } else if flags.contains(Flags::INTEGER) {
                    serializer.serialize_i64(self.iv() as i64)
                } else {
                    serializer.serialize_unit()
                }
            }
            Type::Other(_) => Err(S::Error::custom(
                "cannot deserialize weird magic perl values",
            )),

            // These are impossible as they are all handled by different Value enum types:
            Type::Reference => Value::from(
                self.dereference()
                    .ok_or_else(|| S::Error::custom("failed to dereference perl value"))?,
            )
            .serialize(serializer),
            Type::Array => {
                let this = unsafe { crate::Array::from_raw_ref(self.sv() as *mut ffi::AV) };
                this.serialize(serializer)
            }
            Type::Hash => {
                let this = unsafe { crate::Hash::from_raw_ref(self.sv() as *mut ffi::HV) };
                this.serialize(serializer)
            }
        }
    }
}
