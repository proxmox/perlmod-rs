//! Module containing the [`Scalar`] and [`Mortal`] types.

use std::marker::PhantomData;
use std::mem;

use bitflags::bitflags;

use crate::error::MagicError;
use crate::ffi::{self, SV};
use crate::magic::{Leakable, MagicSpec, MagicValue};
use crate::raw_value;
use crate::{Error, Value};

/// An owned reference to a perl value.
///
/// This keeps a reference to a value which lives in the perl interpreter.
/// This derefs to a [`ScalarRef`] which implements most of the basic functionality common to all
/// [`SV`] related types.
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

    /// Turn this into a raw [`SV`] transferring control of one reference count.
    pub fn into_raw(self) -> *mut SV {
        let ptr = self.0;
        core::mem::forget(self);
        ptr
    }

    /// Create a wrapping [`Scalar`] from an [`SV`] pointer. The [`Scalar`] takes over the owned
    /// reference from the passed [`SV`], which means it must not be a mortal reference.
    ///
    /// # Safety
    ///
    /// This does not change the value's reference count, it is assumed that we're taking ownership
    /// of one reference.
    ///
    /// The caller must ensure that it is safe to decrease the reference count later on, or use
    /// [`into_raw()`](Scalar::into_raw) instead of letting the [`Scalar`] get dropped.
    pub unsafe fn from_raw_move(ptr: *mut SV) -> Self {
        Self(ptr)
    }

    /// Increase the reference count on an [`SV`] pointer.
    ///
    /// # Safety
    ///
    /// The caller may still need to decrease the reference count for the `ptr` source value.
    pub unsafe fn from_raw_ref(ptr: *mut SV) -> Self {
        unsafe { Self::from_raw_move(ffi::RSPL_SvREFCNT_inc(ptr)) }
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
        if s.as_bytes().iter().any(|&b| b >= 0x80) {
            unsafe {
                Self::from_raw_move(ffi::RSPL_newSVpvn_utf8(
                    s.as_bytes().as_ptr() as *const libc::c_char,
                    s.as_bytes().len() as libc::size_t,
                ))
            }
        } else {
            Self::new_bytes(s.as_bytes())
        }
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

    /// Try to produce a substring from an existing "base" value and a `&str`.
    ///
    /// Returns `None` if `substr` is not part of `value` or if `substr` is the empty string.
    pub fn substr_from_str_slice(value: &ScalarRef, substr: &str) -> Option<Scalar> {
        if substr.is_empty() {
            return None;
        }

        let value_bytes = value.pv_bytes();
        let value_beg = value_bytes.as_ptr() as usize;
        let value_end = value_beg + value_bytes.len();
        let value_range = value_beg..value_end;

        let str_bytes = substr.as_bytes();
        let str_beg = str_bytes.as_ptr() as usize;
        let str_end = str_beg + str_bytes.len() - 1;
        if !value_range.contains(&str_beg) || !value_range.contains(&str_end) {
            return None;
        }

        // we just checked the ranges:
        let mut start = unsafe { str_bytes.as_ptr().offset_from(value_bytes.as_ptr()) as usize };
        let mut len = substr.len();

        if unsafe { ffi::RSPL_SvUTF8(value.sv()) } {
            // go from byte offset to code point offset
            len = str_bytes
                .iter()
                .copied()
                .filter(|&b| (b as i8) >= -0x40)
                .count();
            start = value_bytes[..start]
                .iter()
                .copied()
                .filter(|&b| (b as i8) >= -0x40)
                .count();
        }

        Some(unsafe {
            Scalar::from_raw_move(ffi::RSPL_substr(
                ffi::RSPL_SvREFCNT_inc(value.sv()),
                start,
                len,
            ))
        })
    }
}

impl Clone for Scalar {
    #[inline]
    fn clone(&self) -> Self {
        unsafe { Self::from_raw_ref(self.sv()) }
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

/// A reference to a perl value. This is a pre reference type and cannot be constructed manually.
/// It is meant to provide methods common to `Value`, `Scalar`, `Array`, `Hash`, as these are all
/// scalar values under the hood.
pub struct ScalarRef(PhantomData<()>);

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

    /// Get the raw `*mut SV` value for this.
    ///
    /// This does not affect the reference count of this value. This is up to the user.
    pub fn as_raw(&self) -> *mut SV {
        self.sv()
    }

    fn get_type(sv: *mut SV) -> Type {
        unsafe {
            if !ffi::RSPL_is_defined(sv) {
                return Type::Scalar(Flags::empty());
            }

            // These are simple:
            if ffi::RSPL_is_reference(sv) {
                return Type::Reference;
            } else if ffi::RSPL_is_array(sv) {
                return Type::Array;
            } else if ffi::RSPL_is_hash(sv) {
                return Type::Hash;
            }

            // Scalars have flags:
            let flags = ffi::RSPL_type_flags(sv);
            if flags != 0 {
                return Type::Scalar(Flags::from_bits_truncate(flags as u8));
            }

            let ty = ffi::RSPL_svtype(sv);
            if ty == 0 {
                // Looks like undef
                Type::Scalar(Flags::empty())
            } else if ty == ffi::RSPL_PVLV() {
                // We don't support all kinds of magic, but some lvalues are simple:
                // Try to GET the value and then check for definedness.
                ffi::RSPL_SvGETMAGIC(sv);
                if !ffi::RSPL_SvOK(sv) {
                    // This happens when the value points to a non-existing hash element we could
                    // auto-vivify, but we won't:
                    return Type::Scalar(Flags::empty());
                }

                // Otherwise we just try to "recurse", which will work for substrings.
                Self::get_type(ffi::RSPL_LvTARG(sv))
            } else {
                Type::Other(ty as u8)
            }
        }
    }

    /// Get some information about the value's type.
    pub fn ty(&self) -> Type {
        Self::get_type(self.sv())
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

    /// Coerce to a double value. (perlxs `SvNV`).
    pub fn nv(&self) -> f64 {
        unsafe { ffi::RSPL_SvNV(self.sv()) }
    }

    /// Coerce to an integer value. (perlxs `SvIV`).
    pub fn iv(&self) -> isize {
        unsafe { ffi::RSPL_SvIV(self.sv()) }
    }

    /// Coerce to an utf8 string value. (perlxs `SvPVutf8`)
    pub fn pv_string_utf8(&self) -> &str {
        unsafe {
            let mut len: libc::size_t = 0;
            let ptr = ffi::RSPL_SvPVutf8(self.sv(), &mut len) as *const u8;
            std::str::from_utf8_unchecked(std::slice::from_raw_parts(ptr, len))
        }
    }

    /// Coerce to a string without utf8 encoding. (perlxs `SvPV`)
    pub fn pv_bytes(&self) -> &[u8] {
        unsafe {
            let mut len: libc::size_t = 0;
            let ptr = ffi::RSPL_SvPV(self.sv(), &mut len) as *const u8;
            std::slice::from_raw_parts(ptr, len)
        }
    }

    /// Coerce to a byte-string, downgrading from utf-8. (perlxs `SvPVbyte`)
    ///
    /// May fail if there are values which don't fit into bytes in the contained utf-8 string, in
    /// which case `None` is returned.
    pub fn pv_utf8_to_bytes(&self) -> Option<&[u8]> {
        unsafe {
            let mut len: libc::size_t = 0;
            let ptr = ffi::RSPL_SvPVbyte(self.sv(), &mut len) as *const u8;
            if ptr.is_null() {
                return None;
            }
            Some(std::slice::from_raw_parts(ptr, len))
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

    /// Convenience check for `SVt_NULL`
    pub fn is_undef(&self) -> bool {
        0 == unsafe { ffi::RSPL_type_flags(self.sv()) }
    }

    // FIXME: self consuming on a phantom type... this can probably not be useful
    /// Turn this into a [`Value`].
    pub fn into_value(self) -> Value {
        Value::from_scalar(self.clone_ref())
    }

    /// Get the reference type for this value. (Similar to `ref` in perl).
    ///
    /// If `blessed` is true and the value is a blessed reference, the package name will be
    /// returned, otherwise the scalar type (`"SCALAR"`, `"ARRAY"`, ...) will be returned.
    pub fn reftype(&self, blessed: bool) -> &'static str {
        let ptr = unsafe { ffi::RSPL_sv_reftype(self.sv(), i32::from(blessed)) };

        if ptr.is_null() {
            "<UNKNOWN>"
        } else {
            unsafe {
                std::ffi::CStr::from_ptr(ptr)
                    .to_str()
                    .unwrap_or("<NON-UTF8-CLASSNAME>")
            }
        }
    }

    /// Check whether this value is a substring.
    pub fn is_substr(&self) -> bool {
        unsafe {
            self.find_raw_magic(
                Some(ffi::RSPL_PERL_MAGIC_substr()),
                Some(&*ffi::RSPL_vtbl_substr()),
            )
            .is_some()
        }
    }

    /// Create a substring from a string.
    pub fn substr<I>(&self, index: I) -> Result<Scalar, Error>
    where
        I: std::slice::SliceIndex<[u8], Output = [u8]>,
    {
        let bytes = self.pv_bytes();
        let slice: &[u8] = bytes
            .get(index)
            .ok_or_else(|| Error::new("substr with out of bounds range"))?;
        let start = unsafe { slice.as_ptr().offset_from(bytes.as_ptr()) };
        let start = usize::try_from(start).map_err(|_| Error::new("bad substr index"))?;

        Ok(unsafe {
            Scalar::from_raw_move(ffi::RSPL_substr(
                ffi::RSPL_SvREFCNT_inc(self.sv()),
                start,
                slice.len(),
            ))
        })
    }

    /// Attach magic to this value.
    ///
    /// # Safety
    ///
    /// The passed `vtbl` must stay valid for as long as the perl value exists.
    /// It is up to the user to make sure `how` has a valid value. Passing `None` will create a
    /// magic value of type `PERL_MAGIC_ext` for convenience (recommended).
    pub unsafe fn add_raw_magic(
        &self,
        obj: Option<&ScalarRef>,
        how: Option<libc::c_int>,
        vtbl: Option<&ffi::MGVTBL>,
        name: *const libc::c_char,
        namelen: i32,
    ) {
        let _magic_ptr = unsafe {
            ffi::RSPL_sv_magicext(
                self.sv(),
                obj.map(Self::sv).unwrap_or(std::ptr::null_mut()),
                how.unwrap_or_else(|| ffi::RSPL_PERL_MAGIC_ext()),
                vtbl,
                name,
                namelen,
            )
        };
    }

    /// Remove attached magic.
    ///
    /// If `ty` is `None`, a `PERL_MAGIC_ext` magic will be removed.
    ///
    /// # Safety
    ///
    /// It is up to the user that doing this will not crash the perl interpreter.
    pub unsafe fn remove_raw_magic(&self, ty: Option<libc::c_int>, vtbl: Option<&ffi::MGVTBL>) {
        unsafe {
            ffi::RSPL_sv_unmagicext(
                self.sv(),
                ty.unwrap_or_else(|| ffi::RSPL_PERL_MAGIC_ext()),
                vtbl,
            )
        }
    }

    /// Find a magic value, if present.
    ///
    /// If `ty` is `None`, a `PERL_MAGIC_ext` magic will be searched for.
    pub fn find_raw_magic(
        &self,
        ty: Option<libc::c_int>,
        vtbl: Option<&ffi::MGVTBL>,
    ) -> Option<&ffi::MAGIC> {
        unsafe {
            ffi::RSPL_mg_findext(
                self.sv(),
                ty.unwrap_or_else(|| ffi::RSPL_PERL_MAGIC_ext()),
                vtbl,
            )
            .as_ref()
        }
    }

    /// Attach a magic tag to this value. This is a more convenient alternative to using
    /// [`add_raw_magic`](ScalarRef::add_raw_magic()) manually.
    pub fn add_magic<T: Leakable>(&self, spec: MagicValue<'_, '_, 'static, T>) {
        unsafe {
            self.add_raw_magic(
                spec.spec.obj,
                spec.spec.how,
                Some(spec.spec.vtbl),
                spec.ptr.map(Leakable::leak).unwrap_or(std::ptr::null()),
                0,
            )
        }
    }

    /// Find a magic value attached to this perl value.
    ///
    /// # Safety
    ///
    /// It is up to the user to ensure the correct types are used in the provided `MagicSpec`.
    pub fn find_magic<'a, 's, 'm, T: Leakable>(
        &'s self,
        spec: &'m MagicSpec<'static, 'static, T>,
    ) -> Option<&'a T::Pointee> {
        match self.find_raw_magic(spec.how, Some(spec.vtbl)) {
            None => None,
            Some(mg) => {
                assert_eq!(
                    mg.vtbl().map(|v| v as *const _),
                    Some(spec.vtbl as *const _),
                    "Perl_mg_findext misbehaved horribly",
                );

                T::get_ref(mg.ptr())
            }
        }
    }

    /// Remove a magic tag from this value previously added via
    /// [`add_magic`](ScalarRef::add_magic()) and potentially reclaim the contained value of type
    /// `T`.
    ///
    /// When using a "default" magic tag via [`MagicTag::DEFAULT`](crate::magic::MagicTag::DEFAULT)
    /// such as when using the [`declare_magic!`](crate::declare_magic!) macro, removing the magic
    /// implicitly causes perl call the `free` method, therefore in this case this method returns
    /// `None`.
    ///
    /// In case the magic was not found, [`MagicError::NotFound("")`] is returned.
    ///
    /// This does not need to include the object and type information.
    pub fn remove_magic<T: Leakable>(
        &self,
        spec: &MagicSpec<'static, 'static, T>,
    ) -> Result<Option<T>, MagicError> {
        let this = match self.find_raw_magic(spec.how, Some(spec.vtbl)) {
            None => Err(MagicError::NotFound("")),
            Some(mg) => {
                assert_eq!(
                    mg.vtbl().map(|v| v as *const _),
                    Some(spec.vtbl as *const _),
                    "Perl_mg_findext misbehaved horribly",
                );

                Ok(match mg.vtbl() {
                    // We assume that a 'free' callback takes care of reclaiming the value!
                    Some(v) if v.free.is_some() => None,
                    _ => T::get_ref(mg.ptr()).map(|m| unsafe { T::reclaim(m) }),
                })
            }
        };

        unsafe {
            self.remove_raw_magic(spec.how, Some(spec.vtbl));
        }
        this
    }

    /// Merges a `Cow<str>` with this value.
    ///
    /// Note that the `Cow` part is not required here.
    ///
    /// If `self` is a UTF-8 scalar and its memory representation covers the borrowed substring,
    /// this is equivalent to calling [`Scalar::substr`] with the `index` matching the string.
    ///
    /// Otherwise (if the provided string is unrelated to `self`), this is also equivalent to
    /// calling `[Scalar::new_string]`.
    pub fn merge_str_slice(&self, text: &str) -> Scalar {
        Scalar::substr_from_str_slice(self, text).unwrap_or_else(|| Scalar::new_string(text))
    }
}

impl std::fmt::Debug for Scalar {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let this: &ScalarRef = self;
        std::fmt::Debug::fmt(this, f)
    }
}

impl std::fmt::Debug for ScalarRef {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use std::fmt::Debug;
        match self.ty() {
            Type::Scalar(flags) => {
                if flags.intersects(Flags::STRING) {
                    Debug::fmt(self.pv_string_utf8(), f)
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

        if raw_value::is_enabled() {
            return raw_value::serialize_raw(self, serializer);
        }

        match self.ty() {
            Type::Scalar(flags) => {
                if flags.contains(Flags::STRING) {
                    serializer.serialize_str(self.pv_string_utf8())
                } else if flags.contains(Flags::DOUBLE) {
                    serializer.serialize_f64(self.nv())
                } else if flags.contains(Flags::INTEGER) {
                    serializer.serialize_i64(self.iv() as i64)
                } else if flags.is_empty() {
                    serializer.serialize_none()
                } else {
                    serializer.serialize_unit()
                }
            }
            Type::Other(other) => Err(S::Error::custom(format!(
                "cannot serialize weird magic perl values ({})",
                other,
            ))),

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
