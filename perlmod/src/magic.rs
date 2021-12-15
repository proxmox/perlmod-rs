//! Helpers for making attaching magic values more convenient and slightly less unsafe.
//!
//! This is a safer alternative to directly blessing raw pointer values as it is more difficult to
//! get the perl side of this wrong.
//!
//! For instance, one major downside of using `[Value::bless_box]` is that it is possible to simply
//! bless invalid values into the same class, or create clones via perl's `Storable::dclone` or
//! `Clone::clone`, which can easily cause a double-free corruption.
//!
//! To avoid this, we can attach a magic value to a perl value (or even multiple magic values if so
//! desired).
//!
//! Here's an example for a less error prone perl class:
//!
//! ```
//! #[perlmod::package(name = "RSPM::Convenient::Magic", lib = "perlmod_test")]
//! mod export {
//!     use perlmod::{Error, Value};
//!
//!     // This declares `CLASSNAME`, `MAGIC` and adds a `TryFrom<&Value>` implementation for
//!     // `&Magic`.
//!     perlmod::declare_magic!(Box<Magic> : &Magic as "RSPM::Convenient::Magic");
//!
//!     // This is our data type.
//!     struct Magic {
//!         content: String,
//!     }
//!
//!     // We can add a drop handler in rust if we like.
//!     impl Drop for Magic {
//!         fn drop(&mut self) {
//!             # fn code() {}
//!             code();
//!         }
//!     }
//!
//!     #[export(raw_return)] // `raw` and `raw_return` attributes are an optional optimization
//!     fn new(#[raw] class: Value, content: String) -> Result<Value, Error> {
//!         // `instantiate_magic` is a shortcut for the most "common" type of blessed object: a
//!         // hash. We don't actually make use of the hash itself currently (but we could).
//!         Ok(perlmod::instantiate_magic!(&class, MAGIC => Box::new(Magic { content })))
//!     }
//!
//!     // The `declare_magic` macro prepared all we need for the `#[try_from_ref]` attribute to
//!     // work.
//!     #[export]
//!     fn call(#[try_from_ref] this: &Magic) -> Result<(), Error> {
//!         println!("Calling magic with content {:?}", this.content);
//!         Ok(())
//!     }
//! }
//! ```
//!
//! The above allows for the following perl code:
//!
//! ```perl, ignore
//! use RSPM::Convenient::Magic;
//! my $value = RSPM::Convenient::Magic->new("Some Content");
//! $value->call();
//! undef $value; # Here, the DESTROY and `Drop` implementations will be called.
//! ```
//!

use std::marker::PhantomData;

use crate::ffi;
use crate::perl_fn;
use crate::ScalarRef;

/// Pointer-like types which can be leaked and reclaimed.
///
/// # Safety
///
/// Generally it is up to the user to decide how to go on with this.  `leak` and `reclaim` should
/// balance out reference counts, and so forth.
pub unsafe trait Leakable {
    type Pointee;

    /// Leak this value as a pointer.
    fn leak(self) -> *const libc::c_char;

    /// Reclaim a value from a pointer.
    ///
    /// # Safety
    ///
    /// This must only be called *once* on values previously created by calling [`get_ref`] on a
    /// value obtained from [`leak`].
    /// Implementors must ensure that it is always safe to call this exactly *once* on *each*
    /// leaked value.
    ///
    /// [`get_ref`]: Leakable::get_ref
    /// [`leak`]: Leakable::leak
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
pub struct MagicTag<T = ()>(ffi::MGVTBL, PhantomData<T>);

/// It doesn't actually contain a `T`
unsafe impl<T> Sync for MagicTag<T> {}

/// It doesn't actually contain a `T`
unsafe impl<T> Send for MagicTag<T> {}

impl<T> MagicTag<T> {
    /// Create a new tag. See [`MagicSpec`] for its usage.
    pub const fn new() -> Self {
        Self(ffi::MGVTBL::zero(), PhantomData)
    }
}

impl<T> AsRef<ffi::MGVTBL> for MagicTag<T> {
    fn as_ref(&self) -> &ffi::MGVTBL {
        &self.0
    }
}

impl<T: Leakable> MagicTag<T> {
    perl_fn! {
        extern "C" fn drop_handler(_sv: *mut ffi::SV, mg: *mut ffi::MAGIC) -> libc::c_int {
            let mg = unsafe { &*mg };
            match T::get_ref(mg.ptr()) {
                Some(ptr) => {
                    let _drop = unsafe { T::reclaim(ptr) };
                }
                None => eprintln!("Default magic drop handler called but pointer was NULL"),
            }
            0
        }
    }

    /// The default tag, note that using this tag when creating perl values for *different* types
    /// than `T` this *will* cause memory corruption!
    pub const DEFAULT: Self = Self(
        ffi::MGVTBL {
            free: Some(Self::drop_handler),
            get: None,
            set: None,
            len: None,
            clear: None,
            copy: None,
            dup: None,
            local: None,
        },
        PhantomData,
    );
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
#[derive(Clone)]
pub struct MagicSpec<'o, 'v, T> {
    pub(crate) obj: Option<&'o ScalarRef>,
    pub(crate) how: Option<libc::c_int>,
    pub(crate) vtbl: &'v ffi::MGVTBL,
    _phantom: PhantomData<T>,
}

/// It doesn't actually contain a `T`
unsafe impl<'o, 'v, T> Sync for MagicSpec<'o, 'v, T> {}

/// It doesn't actually contain a `T`
unsafe impl<'o, 'v, T> Send for MagicSpec<'o, 'v, T> {}

impl<T> MagicSpec<'static, 'static, T> {
    /// Create a new static magic specification from a tag.
    ///
    /// # Safety
    ///
    /// This should be safe as long as the [`MagicTag`] is only used for a single [`MagicSpec`].
    pub const unsafe fn new_static<TT>(vtbl: &'static MagicTag<TT>) -> Self {
        Self {
            obj: None,
            how: None,
            vtbl: &vtbl.0,
            _phantom: PhantomData,
        }
    }

    /// Get the minimum required for [`remove_magic`](ScalarRef::remove_magic()).
    pub const fn spec(&self) -> MagicSpec<'static, 'static, T> {
        MagicSpec {
            obj: None,
            how: self.how,
            vtbl: self.vtbl,
            _phantom: PhantomData,
        }
    }
}

impl<'o, 'v, T: Leakable> MagicSpec<'o, 'v, T> {
    /// Leak a `T` into a new `MagicSpec`.
    ///
    /// This LEAKS.
    pub fn with_value<'a>(&'a self, ptr: T) -> MagicValue<'a, 'o, 'v, T> {
        MagicValue {
            spec: self,
            ptr: Some(ptr),
        }
    }
}

/// We want to instantiate `MagicSpec` as `static`, because as `const` the contained values may not
/// actually end up to be guaranteed the same storage everywhere the `const` is accessed. But in
/// order to be able to create a `static`, it must be `Sync`, so `MagicSpec` does not contain the
/// pointer value anymore, instead, a `MagicValue` is instantiated for this.
pub struct MagicValue<'spec, 'o, 'v, T> {
    pub(crate) spec: &'spec MagicSpec<'o, 'v, T>,
    pub(crate) ptr: Option<T>,
}
