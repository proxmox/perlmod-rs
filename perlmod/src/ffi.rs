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

// in our glue:
#[link(name = "glue", kind = "static")]
extern "C" {
    pub fn RSPL_StackMark_count(this: usize) -> usize;

    pub fn RSPL_stack_get(offset: usize) -> *mut SV;

    pub fn RSPL_croak_sv(sv: *mut SV) -> !;
    pub fn RSPL_SvNV(sv: *mut SV) -> f64;
    pub fn RSPL_SvIV(sv: *mut SV) -> isize;
    pub fn RSPL_SvPVutf8(sv: *mut SV, len: *mut libc::size_t) -> *const libc::c_char;
    pub fn RSPL_SvPV(sv: *mut SV, len: *mut libc::size_t) -> *const libc::c_char;
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
    pub fn RSPL_SvTRUE(sv: *mut SV) -> bool;

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

    pub fn RSPL_SvFLAGS(sv: *mut SV) -> u32;
    pub fn RSPL_SvGETMAGIC(sv: *mut SV) -> bool;
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
        RSPL_stack_shrink_to(self.0);
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
    StackMark(RSPL_pop_markstack_ptr())
}

/// Push a value to the stack.
///
/// # Safety
///
/// Read up on mortals and the stack and when it is legal to put a value onto it. Typically a
/// mortal value with no more references to it to avoid leaking if they aren't used later on.
pub unsafe fn stack_push_raw(value: *mut SV) {
    RSPL_stack_resize_by(1);
    *RSPL_stack_sp() = value;
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
/// # #[derive(Serialize)]
/// # struct Output;
///
/// # fn code_to_extract_parameters() {}
/// # fn actual_rust_function(_arg: ()) -> Result<Output, String> { Ok(Output) }
/// #[no_mangle]
/// pub extern "C" fn exported_name(cv: &::perlmod::ffi::CV) {
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
    RSPL_croak_sv(sv);
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
