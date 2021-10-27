//! Helpers for making attaching magic values more convenient and slightly less unsafe.

use crate::ffi;
use crate::ScalarRef;

/// Pointer-like types which can be leaked and reclaimed.
///
/// # Safety
///
/// Generally it is up to the user to decide how to go on with this.  `leak` and `reclaim` should
/// balance out reference counts, and so forth.
pub unsafe trait Leakable {
    type Pointee;
    fn leak(self) -> *const libc::c_char;
    unsafe fn reclaim(ptr: &Self::Pointee) -> Self;
    fn get_ref<'a>(ptr: *const libc::c_char) -> Option<&'a Self::Pointee> {
        unsafe { (ptr as *const Self::Pointee).as_ref() }
    }
}

unsafe impl<T> Leakable for Box<T> {
    type Pointee = T;

    fn leak(self) -> *const libc::c_char {
        Box::leak(self) as *mut T as *const T as *const libc::c_char
    }

    unsafe fn reclaim(ptr: &T) -> Self {
        Box::from_raw(ptr as *const T as *mut T)
    }
}

unsafe impl<T> Leakable for std::sync::Arc<T> {
    type Pointee = T;

    fn leak(self) -> *const libc::c_char {
        std::sync::Arc::into_raw(self) as *const libc::c_char
    }

    unsafe fn reclaim(ptr: &T) -> Self {
        std::sync::Arc::from_raw(ptr as *const T)
    }
}

unsafe impl<T> Leakable for std::rc::Rc<T> {
    type Pointee = T;

    fn leak(self) -> *const libc::c_char {
        std::rc::Rc::into_raw(self) as *const libc::c_char
    }

    unsafe fn reclaim(ptr: &T) -> Self {
        std::rc::Rc::from_raw(ptr as *const T)
    }
}

/// A tag for perl magic, see [`MagicSpec`] for its usage.
pub struct MagicTag(ffi::MGVTBL);

impl MagicTag {
    /// Create a new tag. See [`MagicSpec`] for its usage.
    pub const fn new() -> Self {
        Self(ffi::MGVTBL::zero())
    }
}

impl AsRef<ffi::MGVTBL> for MagicTag {
    fn as_ref(&self) -> &ffi::MGVTBL {
        &self.0
    }
}

/// A tag for perl magic. Use this for blessed objects.
///
/// When creating a blessed object is safer to attach the rust pointer via magic than by embedding
/// it in the value itself, since APIs like `Storable::dclone` or `Clone::clone` will perform a raw
/// copy of the pointer and potentially call the destructor twice, potentially leading to double
/// free corruptions.
///
/// Usage example:
/// ```
/// #[perlmod::package(name = "RSPM::Doc::Magic")]
/// mod export {
///     use perlmod::{Error, Value};
///
///     const CLASSNAME: &str = "RSPM::Doc::Magic";
///     const MAGIC: perlmod::MagicSpec<Box<MyData>> = unsafe {
///         // Safety: MagicTag used only once here:
///         const TAG: perlmod::MagicTag = perlmod::MagicTag::new();
///         perlmod::MagicSpec::new_static(&TAG)
///     };
///
///     struct MyData(String);
///
///     #[export(raw_return)]
///     fn new(#[raw] class: Value) -> Result<Value, Error> {
///         let this = Value::new_hash();
///         let this = Value::new_ref(&this);
///         // bless first:
///         this.bless_sv(&class)?;
///         // then attach the value
///         this.add_magic(MAGIC.with_value(
///             Box::new(MyData(format!("Hello")))
///         ));
///         Ok(this)
///     }
///
///     #[export]
///     fn destroy(#[raw] this: Value) {
///         perlmod::magic_destructor!(this: &MAGIC);
///     }
/// }
/// ```
///
/// NOTE: Once `const fn` with trait bounds are stable, this will be `where T: Leakable`.
pub struct MagicSpec<'o, 'v, T> {
    pub(crate) obj: Option<&'o ScalarRef>,
    pub(crate) how: Option<libc::c_int>,
    pub(crate) vtbl: &'v ffi::MGVTBL,
    pub(crate) ptr: Option<T>,
}

impl<T> MagicSpec<'static, 'static, T> {
    /// Create a new static magic specification from a tag.
    ///
    /// # Safety
    ///
    /// This should be safe as long as the [`MagicTag`] is only used for a single [`MagicSpec`].
    pub const unsafe fn new_static(vtbl: &'static MagicTag) -> Self {
        Self {
            obj: None,
            how: None,
            vtbl: &vtbl.0,
            ptr: None,
        }
    }

    /// Get the minimum required for [`remove_magic`](ScalarRef::remove_magic()).
    pub const fn spec(&self) -> MagicSpec<'static, 'static, T> {
        MagicSpec {
            obj: None,
            how: self.how,
            vtbl: self.vtbl,
            ptr: None,
        }
    }
}

impl<'o, 'v, T: Leakable> MagicSpec<'o, 'v, T> {
    /// Leak a `T` into a new `MagicSpec`.
    ///
    /// This LEAKS.
    pub fn with_value(&self, ptr: T) -> Self {
        MagicSpec {
            obj: self.obj.clone(),
            how: self.how.clone(),
            vtbl: self.vtbl,
            ptr: Some(ptr),
        }
    }
}
