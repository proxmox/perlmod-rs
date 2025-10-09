//! Crate for creating perl packages/bindings for rust code.
//!
//! The main feature of this crate is the [`package`] macro provided by the `perlmod-macro` crate
//! and documented here.
//!
//! The underlying machinery for these macros is contained in this crate and provides ways to
//! serialize and deserialize data between perl and rust.
//!
//! # Blessed Values
//!
//! This crate also provides all the tools required to bless perl values into packages, and
//! associate rust data with perl values.
//! There are multiple ways to do this, and they come with different issues.
//!
//! The currently recommended way is found in the documentation of the [`magic`] module, which is
//! considered the least error prone.
//!
//! A less safe (and lower-level) example can be found in the documentation of the
//! [`Value::bless`](Value::bless()) method.
//!
//! [`package`]: attr.package.html
//! [`export`]: attr.export.html

#![deny(unsafe_op_in_unsafe_fn)]

pub mod error;
pub use error::Error;

#[macro_use]
mod macros;

#[macro_use]
pub mod ffi;
#[doc(inline)]
pub use ffi::Gimme;

pub mod de;
pub mod ser;

#[doc(inline)]
pub use de::{from_ref_value, from_value};
#[doc(inline)]
pub use ser::to_value;

pub mod scalar;
#[doc(inline)]
pub use scalar::{Mortal, Scalar, ScalarRef};

pub mod array;
#[doc(inline)]
pub use array::Array;

pub mod hash;
#[doc(inline)]
pub use hash::Hash;

pub mod value;
#[doc(inline)]
pub use value::Value;

pub(crate) mod raw_value;
pub use raw_value::RawValue;

pub mod magic;
#[doc(inline)]
pub use magic::{MagicSpec, MagicTag, MagicValue};

#[cfg(feature = "exporter")]
#[doc(inline)]
pub use perlmod_macro::package;

#[cfg(feature = "exporter")]
#[doc(inline)]
/// Attribute to export a function so that it can be installed as an `xsub` in perl. See the
/// [`package!`](macro@package) macro for a usage example.
///
/// This macro has the following optional arguments:
///
/// * `raw_return`: specifies that the return type, which must be a [`Value`], will be returned as
///   is, and not go through serialization. As of perlmod
///   0.6, serialization of a [`Value`] will not produce a clone, so this is mostly an
///   optimization.
/// * `prototype`: The perl prototype for the function. By default, this will be guessed from the
///   parameters as a chain of '$', with trailing `Option<>` parameters behind a `;`. So for
///   example, an `fn(i32, Option<i32>, i32, Option<i32>)` has the prototype `$$$;$`.
/// * `xs_name`: override the name of the exported xsub, this is not recommended and only makes
///   sense when *not* using the `#[package]` macro, as with the `#[package]` macro, these aren't
///   publicly visible.
/// * `name`: the name the function should be using in perl. This only makes sense with the
///   `#[package]` macro, as otherwise the user is responsible for loading the function via perl's
///   `DynaLoader` on their own.
/// * `errno`: copy the value set via [`set_errno`](crate::error::set_errno) to libc's `errno`
///   location before returning to perl (after all side effects such as destructors) have run, in
///   order to allow setting perl's `$!` variable.
/// * `serialize_error`: Instead of stringifying the `Err` part of a `Result` via `Display`,
///   serialize it into a structured value.
///
/// Additionally, function parameters can also use the following attributes:
///
/// * `#[raw]` with a parameter of type [`Value`]: The parameter will be passed as
///   is and not go through deserialization. As of perlmod 0.6, deserialization will not produce
///   clones anymore, so this is mostly an optimization.
/// * `#[try_from_ref]`: Instead of regular deserialization, `TryFrom::try_from(&Value)` will be
///   used.
///
///   Implementing the `TryFrom` trait accordingly can make using blessed references more
///   convenient, but at the cost of hiding underlying `unsafe` code.
///
/// * `#[cv]`: This can be used on a single parameter of type [`&CV`](ffi::CV) to get
///   access to the `xsub` value used to call the function.
///
///   This can be used, for instance, to associate callables with data, or to create attach boxed
///   closures with an xsub as an entry point to retrieving the closure via
///   [`magic`](ScalarRef::add_magic).
///
/// For an example on making blessed objects, see [`Value::bless_box`](Value::bless_box()).
pub use perlmod_macro::export;

mod elf_notes;

#[doc(hidden)]
pub mod __private__ {
    //! This is private and not meant to be a public API and thus semver exempt.

    pub use super::elf_notes::ElfNote;
}

/// Shortcut for `Gimme::get() == Gimme::List`.
pub fn wantarray() -> bool {
    Gimme::get() == Gimme::List
}
