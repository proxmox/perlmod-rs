//! Unsafe ffi code.
//!
//! You should not use this code directly. This is used by the binding generator to implement xsubs
//! for exported functions.

/// Raw perl subroutine pointer value. This should not be used directly.
#[repr(C)]
pub struct CV {
    _ffi: usize,
}

/// Raw scalar-ish perl value. This should not be used directly.
#[repr(C)]
pub struct SV {
    _ffi: usize,
}

/// Raw perl array value. This should not be used directly.
#[repr(C)]
pub struct AV {
    _ffi: usize,
}

/// Raw perl hash value. This should not be used directly.
#[repr(C)]
pub struct HV {
    _ffi: usize,
}

/// Raw perl hash entry iterator. This should not be used directly.
#[repr(C)]
pub struct HE {
    _ffi: usize,
}

/// Raw perl MAGIC struct, we don't actually make its contents available.
#[repr(C)]
pub struct MAGIC {
    _ffi: usize,
}

#[allow(clippy::len_without_is_empty)]
impl MAGIC {
    pub fn vtbl(&self) -> Option<&MGVTBL> {
        unsafe { RSPL_MAGIC_virtual(self as *const MAGIC).as_ref() }
    }

    pub fn ptr(&self) -> *const libc::c_char {
        unsafe { RSPL_MAGIC_ptr(self as *const MAGIC) }
    }

    pub fn len(&self) -> isize {
        unsafe { RSPL_MAGIC_len(self as *const MAGIC) }
    }
}

#[repr(C)]
pub struct Unsupported {
    _ffi: usize,
}

#[cfg(perlmod = "multiplicity")]
#[repr(C)]
pub struct Interpreter {
    _ffi: usize,
}

/// Build perl-compatible functions and fn types (`pTHX` macro equivalent).
///
/// Takes an `extern "C" fn` (with or without body) and potentially inserts the a
/// `*const Interpreter` as first parameter depending on the perl configuration, so it can be used
/// for xsub implementations.
#[macro_export]
macro_rules! perl_fn {
    // inherited visibility
    ($(
        $(#[$attr:meta])*
        extern "C" fn($($args:tt)*) $(-> $re:ty)?
    )*) => {
        $crate::perl_fn_impl! {
            $(
                $(#[$attr])*
                () extern "C" fn($($args)*) $(-> $re)?
            )*
        }
    };
    ($(
        $(#[$attr:meta])*
        extern "C" fn $name:ident $(<($($gen:tt)*)>)? ($($args:tt)*) $(-> $re:ty)?
        $(where ($($where_clause:tt)*))?
        {
            $($code:tt)*
        }
    )*) => {
        $crate::perl_fn_impl! {
            $(
                $(#[$attr])*
                () extern "C" fn $name $(<($($gen)*)>)? ($($args)*) $(-> $re)?
                $(where ($($where_clause)*))?
                {
                    $($code)*
                }
            )*
        }
    };

    // same with 'pub' visibility
    ($(
        $(#[$attr:meta])*
        pub $(($($vis:tt)+))? extern "C" fn($($args:tt)*) $(-> $re:ty)?
    )*) => {
        $crate::perl_fn_impl! {
            $(
                $(#[$attr])*
                (pub $(($($vis)+))?) extern "C" fn($($args)*) $(-> $re)?
            )*
        }
    };
    ($(
        $(#[$attr:meta])*
        pub $(($($vis:tt)+))?
        extern "C" fn $name:ident $(<($($gen:tt)*)>)? ($($args:tt)*) $(-> $re:ty)?
        $(where ($($where_clause:tt)*))?
        {
            $($code:tt)*
        }
    )*) => {
        $crate::perl_fn_impl! {
            $(
                $(#[$attr])*
                (pub $(($($vis)+))?) extern "C" fn $name $(<($($gen)*)>)? ($($args)*) $(-> $re)?
                $(where ($($where_clause)*))?
                {
                    $($code)*
                }
            )*
        }
    };
}

#[cfg(perlmod = "multiplicity")]
mod vtbl_types_impl {
    use super::{Interpreter, MAGIC, SV};
    use libc::c_int;

    pub type Get = extern "C" fn(_perl: *const Interpreter, sv: *mut SV, mg: *mut MAGIC) -> c_int;
    pub type Set = extern "C" fn(_perl: *const Interpreter, sv: *mut SV, mg: *mut MAGIC) -> c_int;
    pub type Len = extern "C" fn(_perl: *const Interpreter, sv: *mut SV, mg: *mut MAGIC) -> u32;
    pub type Clear = extern "C" fn(_perl: *const Interpreter, sv: *mut SV, mg: *mut MAGIC) -> c_int;
    pub type Free = extern "C" fn(_perl: *const Interpreter, sv: *mut SV, mg: *mut MAGIC) -> c_int;
    pub type Copy = extern "C" fn(
        _perl: *const Interpreter,
        sv: *mut SV,
        mg: *mut MAGIC,
        nsv: *mut SV,
        name: *const libc::c_char,
        namelen: i32,
    ) -> c_int;
    pub type Dup = extern "C" fn(
        _perl: *const Interpreter,
        sv: *mut SV,
        mg: *mut MAGIC,
        clone_parms: *mut super::Unsupported,
    ) -> c_int;
    pub type Local = extern "C" fn(_perl: *const Interpreter, sv: *mut SV, mg: *mut MAGIC) -> c_int;

    #[doc(hidden)]
    #[macro_export]
    macro_rules! perl_fn_impl {
        ($(
            $(#[$attr:meta])*
            ($($vis:tt)*) extern "C" fn($($args:tt)*) $(-> $re:ty)?
        )*) => {$(
            $(#[$attr])*
            $($vis)* extern "C" fn(*const $crate::ffi::Interpreter, $($args)*) $(-> $re)?
        )*};
        ($(
            $(#[$attr:meta])*
            ($($vis:tt)*)
            extern "C" fn $name:ident $(<($($gen:tt)*)>)? ($($args:tt)*) $(-> $re:ty)?
            $(where ($($where_clause:tt)*))?
            {
                $($code:tt)*
            }
        )*) => {$(
            $(#[$attr])*
            $($vis)* extern "C" fn $name $(<$($gen)*>)? (
                _perl: *const $crate::ffi::Interpreter,
                $($args)*
            ) $(-> $re)?
            $(where $($where_clause)*)?
            {
                $($code)*
            }
        )*};
    }
}

#[cfg(not(perlmod = "multiplicity"))]
mod vtbl_types_impl {
    use super::{Interpreter, MAGIC, SV};
    use libc::c_int;

    pub type Get = extern "C" fn(sv: *mut SV, mg: *mut MAGIC) -> c_int;
    pub type Set = extern "C" fn(sv: *mut SV, mg: *mut MAGIC) -> c_int;
    pub type Len = extern "C" fn(sv: *mut SV, mg: *mut MAGIC) -> u32;
    pub type Clear = extern "C" fn(sv: *mut SV, mg: *mut MAGIC) -> c_int;
    pub type Free = extern "C" fn(sv: *mut SV, mg: *mut MAGIC) -> c_int;
    pub type Copy = extern "C" fn(
        sv: *mut SV,
        mg: *mut MAGIC,
        nsv: *mut SV,
        name: *const libc::c_char,
        namelen: i32,
    ) -> c_int;
    pub type Dup =
        extern "C" fn(sv: *mut SV, mg: *mut MAGIC, clone_parms: *mut super::Unsupported) -> c_int;
    pub type Local = extern "C" fn(sv: *mut SV, mg: *mut MAGIC) -> c_int;

    #[doc(hidden)]
    #[macro_export]
    macro_rules! perl_fn_impl {
        ($(
            $(#[$attr:meta])*
            ($($vis:tt)*) extern "C" fn($($args:tt)*) $(-> $re:ty)?
        )*) => {$(
            $(#[$attr])*
            $($vis)* extern "C" fn($($args)*) $(-> $re)?
        )*};
        ($(
            $(#[$attr:meta])*
            ($($vis:tt)*)
            extern "C" fn $name:ident $(<($($gen:tt)*)>)? ($($args:tt)*) $(-> $re:ty)?
            $(where ($($where_clause:tt)*))?
            {
                $($code:tt)*
            }
        )*) => {$(
            $(#[$attr])*
            $($vis)* extern "C" fn $name $(<$($gen)*>)? ($($args)*) $(-> $re)?
            $(where $($where_clause)*)?
            {
                $($code)*
            }
        )*};
    }
}

/// The types in this module depend on the configuration of your perl installation.
///
/// If the perl interpreter has been compiled with `USEMULTIPLICITY`, these methods have an
/// additional parameter.
pub mod vtbl_types {
    pub use super::vtbl_types_impl::*;
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct MGVTBL {
    pub get: Option<vtbl_types::Get>,
    pub set: Option<vtbl_types::Set>,
    pub len: Option<vtbl_types::Len>,
    pub clear: Option<vtbl_types::Clear>,
    pub free: Option<vtbl_types::Free>,
    pub copy: Option<vtbl_types::Copy>,
    pub dup: Option<vtbl_types::Dup>,
    pub local: Option<vtbl_types::Local>,
}

impl MGVTBL {
    /// Let's not expose this directly, we need there to be distinct instances of these, so they
    /// should be created via `MGVTBL::zero()`.
    const EMPTY: Self = Self {
        get: None,
        set: None,
        len: None,
        clear: None,
        free: None,
        copy: None,
        dup: None,
        local: None,
    };

    /// Create a new all-zeroes vtbl as perl docs suggest this is the safest way to
    /// make sure what a `PERL_MAGIC_ext` magic actually means, as the ptr value
    /// may be arbitrary.
    ///
    /// # Safety
    ///
    /// This must not be deallocated as long as it is attached to a perl value, so best use this as
    /// `const` variables, rather than dynamically allocating it.
    pub const fn zero() -> Self {
        Self::EMPTY
    }
}

// in our glue:
#[link(name = "glue", kind = "static")]
unsafe extern "C" {
    pub fn RSPL_StackMark_count(this: usize) -> usize;

    pub fn RSPL_stack_get(offset: usize) -> *mut SV;

    pub fn RSPL_croak_sv(sv: *mut SV) -> !;

    pub fn RSPL_newXS_flags(
        name: *const i8,
        subaddr: *const i8,
        filename: *const i8,
        proto: *const i8,
        flags: u32,
    ) -> *mut CV;

    pub fn RSPL_SvNV(sv: *mut SV) -> f64;
    pub fn RSPL_SvIV(sv: *mut SV) -> isize;
    pub fn RSPL_SvPVutf8(sv: *mut SV, len: *mut libc::size_t) -> *const libc::c_char;
    pub fn RSPL_SvPV(sv: *mut SV, len: *mut libc::size_t) -> *const libc::c_char;
    pub fn RSPL_SvUTF8(sv: *mut SV) -> bool;
    /// This calls `sv_utf8_downgrade` first to avoid croaking, instead returns `NULL` on error.
    pub fn RSPL_SvPVbyte(sv: *mut SV, len: *mut libc::size_t) -> *const libc::c_char;
    pub fn RSPL_sv_2mortal(sv: *mut SV) -> *mut SV;
    pub fn RSPL_get_undef() -> *mut SV;
    pub fn RSPL_get_yes() -> *mut SV;
    pub fn RSPL_get_no() -> *mut SV;
    pub fn RSPL_pop_markstack_ptr() -> usize;
    pub fn RSPL_stack_resize_by(count: isize);
    pub fn RSPL_stack_shrink_to(count: usize);
    pub fn RSPL_stack_sp() -> *mut *mut SV;
    pub fn RSPL_newRV_inc(sv: *mut SV) -> *mut SV;
    pub fn RSPL_newSViv(v: isize) -> *mut SV;
    pub fn RSPL_newSVuv(v: usize) -> *mut SV;
    pub fn RSPL_newSVnv(v: f64) -> *mut SV;
    pub fn RSPL_newSVpvn(v: *const libc::c_char, len: libc::size_t) -> *mut SV;
    pub fn RSPL_newSVpvn_utf8(v: *const libc::c_char, len: libc::size_t) -> *mut SV;
    pub fn RSPL_SvREFCNT_inc(sv: *mut SV) -> *mut SV;
    pub fn RSPL_SvREFCNT_dec(sv: *mut SV);
    pub fn RSPL_is_reference(sv: *mut SV) -> bool;
    pub fn RSPL_dereference(sv: *mut SV) -> *mut SV;
    pub fn RSPL_is_array(sv: *mut SV) -> bool;
    pub fn RSPL_is_hash(sv: *mut SV) -> bool;
    pub fn RSPL_type_flags(sv: *mut SV) -> u32;
    pub fn RSPL_svtype(sv: *mut SV) -> u32;
    pub fn RSPL_SvOK(sv: *mut SV) -> bool;
    pub fn RSPL_SvANY(sv: *mut SV) -> bool;
    pub fn RSPL_SvTRUE(sv: *mut SV) -> bool;

    pub fn RSPL_is_defined(sv: *mut SV) -> bool;

    pub fn RSPL_newAV() -> *mut AV;
    pub fn RSPL_av_extend(av: *mut AV, len: libc::ssize_t);
    pub fn RSPL_av_push(av: *mut AV, sv: *mut SV);
    pub fn RSPL_av_pop(av: *mut AV) -> *mut SV;
    pub fn RSPL_av_len(av: *mut AV) -> usize;
    pub fn RSPL_av_fetch(av: *mut AV, index: libc::ssize_t, lval: i32) -> *mut *mut SV;

    pub fn RSPL_newHV() -> *mut HV;
    pub fn RSPL_HvTOTALKEYS(hv: *mut HV) -> usize;
    pub fn RSPL_hv_fetch(
        hv: *mut HV,
        key: *const libc::c_char,
        klen: i32,
        lval: i32,
    ) -> *mut *mut SV;
    /// Always consumes ownership of `value`.
    pub fn RSPL_hv_store(hv: *mut HV, key: *const libc::c_char, klen: i32, value: *mut SV) -> bool;
    pub fn RSPL_hv_store_ent(hv: *mut HV, key: *mut SV, value: *mut SV) -> bool;
    pub fn RSPL_hv_iterinit(hv: *mut HV);
    pub fn RSPL_hv_iternextsv(
        hv: *mut HV,
        key: *mut *mut libc::c_char,
        retlen: *mut i32,
    ) -> *mut SV;
    pub fn RSPL_hv_iternext(hv: *mut HV) -> *mut HE;
    pub fn RSPL_hv_iterkeysv(he: *mut HE) -> *mut SV;
    pub fn RSPL_hv_iterval(hv: *mut HV, he: *mut HE) -> *mut SV;

    pub fn RSPL_gv_stashsv(name: *const SV, flags: i32) -> *mut HV;
    pub fn RSPL_sv_bless(sv: *mut SV, stash: *mut HV) -> *mut SV;

    pub fn RSPL_ENTER();
    pub fn RSPL_SAVETMPS();
    pub fn RSPL_FREETMPS();
    pub fn RSPL_LEAVE();

    pub fn RSPL_sv_reftype(sv: *const SV, ob: libc::c_int) -> *const libc::c_char;

    pub fn RSPL_PVLV() -> u32;
    pub fn RSPL_LvTARG(sv: *mut SV) -> *mut SV;
    //pub fn RSPL_LvTYPE(sv: *mut SV) -> u8;
    pub fn RSPL_vivify_defelem(sv: *mut SV);

    //pub fn RSPL_SvFLAGS(sv: *mut SV) -> u32;
    pub fn RSPL_SvGETMAGIC(sv: *mut SV) -> bool;

    pub fn RSPL_sv_magicext(
        sv: *mut SV,
        obj: *mut SV,
        how: libc::c_int,
        vtbl: Option<&MGVTBL>,
        name: *const libc::c_char,
        namelen: i32,
    ) -> *mut MAGIC;
    pub fn RSPL_sv_unmagicext(sv: *mut SV, ty: libc::c_int, vtbl: Option<&MGVTBL>);
    pub fn RSPL_mg_findext(sv: *const SV, ty: libc::c_int, vtbl: Option<&MGVTBL>) -> *const MAGIC;
    pub fn RSPL_MAGIC_virtual(mg: *const MAGIC) -> *const MGVTBL;
    pub fn RSPL_MAGIC_ptr(mg: *const MAGIC) -> *const libc::c_char;
    pub fn RSPL_MAGIC_len(mg: *const MAGIC) -> isize;
    pub fn RSPL_PERL_MAGIC_ext() -> libc::c_int;

    pub fn RSPL_PERL_MAGIC_substr() -> libc::c_int;
    pub fn RSPL_vtbl_substr() -> *const MGVTBL;
    pub fn RSPL_substr(orig: *mut SV, off: usize, len: usize) -> *mut SV;

    pub fn RSPL_defstash() -> *mut HV;

    pub fn RSPL_set_use_safe_putenv(on: libc::c_int);
}

/// Argument marker for the stack.
pub struct StackMark(usize);

impl StackMark {
    pub fn count(&self) -> usize {
        unsafe { RSPL_StackMark_count(self.0) }
    }

    pub fn iter(&self) -> StackIter {
        StackIter {
            at: self.0 + 1,
            end: self.0 + 1 + self.count(),
        }
    }

    /// Shrink the perl stack to this mark.
    ///
    /// # Safety
    ///
    /// This is only valid if the mark is still valid (smaller than `PL_stack_sp`) and all values
    /// still remaining on the stack are mortal (which should normally be the case anyway).
    pub unsafe fn set_stack(self) {
        unsafe {
            RSPL_stack_shrink_to(self.0);
        }
    }
}

/// Iterator over the stack up to the [`StackMark`].
pub struct StackIter {
    at: usize,
    end: usize,
}

impl Iterator for StackIter {
    type Item = crate::Scalar;

    fn next(&mut self) -> Option<Self::Item> {
        let at = self.at;
        if at == self.end {
            return None;
        }
        unsafe {
            let ptr = RSPL_stack_get(self.at);
            self.at += 1;
            if ptr.is_null() {
                None
            } else {
                Some(crate::Scalar::from_raw_ref(ptr))
            }
        }
    }
}

/// Pop the current argument marker off of the argument marker stack.
///
/// # Safety
///
/// Read up on `PL_markstack_ptr` in perlguts. This is equivalent to `*PL_markstack_ptr--` in C.
pub unsafe fn pop_arg_mark() -> StackMark {
    StackMark(unsafe { RSPL_pop_markstack_ptr() })
}

/// Push a value to the stack.
///
/// # Safety
///
/// Read up on mortals and the stack and when it is legal to put a value onto it. Typically a
/// mortal value with no more references to it to avoid leaking if they aren't used later on.
pub unsafe fn stack_push_raw(value: *mut SV) {
    unsafe {
        RSPL_stack_resize_by(1);
        *RSPL_stack_sp() = value;
    }
}

pub fn stack_push(value: crate::Mortal) {
    unsafe {
        stack_push_raw(value.into_raw());
    }
}

/// This calls perl's `croak_sv`.
///
/// # Safety
///
/// This seems to perform a `longjmp` and is thus never truly safe in rust code. You really want to
/// limit this to the top entry point of your rust call stack in a separate `extern "C" fn` where
/// no rust values with `Drop` handlers or anything similar are active.
///
/// The `perlmod_macro`'s `export` attribute typically creates 2 wrapper functions of the form:
///
/// ```no_run
/// # use serde::Serialize;
///
/// # struct Output;
/// # impl Serialize for Output {
/// #     fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
/// #         serializer.serialize_unit()
/// #     }
/// # }
///
/// # fn code_to_extract_parameters() {}
/// # fn actual_rust_function(_arg: ()) -> Result<Output, String> { Ok(Output) }
/// #[no_mangle]
/// pub extern "C" fn exported_name(/* pTHX parameter, */ cv: &::perlmod::ffi::CV) {
///     unsafe {
///         match private_implementation_name(cv) {
///             Ok(sv) => ::perlmod::ffi::stack_push_raw(sv),
///             Err(sv) => ::perlmod::ffi::croak(sv),
///         }
///     }
/// }
///
/// #[inline(never)]
/// fn private_implementation_name(
///     _cv: &::perlmod::ffi::CV,
/// ) -> Result<*mut ::perlmod::ffi::SV, *mut ::perlmod::ffi::SV> {
///     let args = code_to_extract_parameters();
///     // ...
///     let result = match actual_rust_function(args) {
///         Ok(output) => output,
///         Err(err) => {
///             return Err(::perlmod::Value::new_string(&err.to_string())
///                 .into_mortal()
///                 .into_raw());
///         }
///     };
///
///     match ::perlmod::to_value(&result) {
///         Ok(value) => Ok(value.into_mortal().into_raw()),
///         Err(err) => Err(::perlmod::Value::new_string(&err.to_string())
///             .into_mortal()
///             .into_raw()),
///     }
/// }
/// ```
pub unsafe fn croak(sv: *mut SV) -> ! {
    unsafe {
        RSPL_croak_sv(sv);
    }
}

/// Create a pseudo-block for mortals & temps to be freed after it.
/// This calls `ENTER; SAVETMPS;` before and `FREETMPS; LEAVE;` after the provided closure.
pub fn pseudo_block<F, R>(func: F) -> R
where
    F: FnOnce() -> R,
{
    unsafe {
        RSPL_ENTER();
        RSPL_SAVETMPS();
    }

    let res = func();

    unsafe {
        RSPL_FREETMPS();
        RSPL_LEAVE();
    }

    res
}

/// Tell perl to use a "safe" `putenv` call instead of manually manipulating the `environ`
/// variable. Without this, changing environment variables can lead to crashes.
pub fn use_safe_putenv(on: bool) {
    unsafe { RSPL_set_use_safe_putenv(on as _) }
}
