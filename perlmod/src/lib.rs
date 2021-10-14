//! Crate for creating perl packages/bindings for rust code.
//!
//! The main feature of this crate is the [`package`] macro provided by the `perlmod-macro` crate
//! and documented here.
//!
//! The underlying machinery for these macros is contained in this crate and provides ways to
//! serialize and deserialize data between perl and rust.
//!
//! [`package`]: attr.package.html
//! [`export`]: attr.export.html

pub(crate) mod error;
pub use error::Error;

#[macro_use]
mod macros;

pub mod de;
pub mod ffi;
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

#[cfg(feature = "exporter")]
#[doc(inline)]
pub use perlmod_macro::package;

#[cfg(feature = "exporter")]
#[doc(inline)]
/// Attribute to export a function so that it can be installed as an `xsub` in perl. See the
/// [`package!`](macro@package) macro for a usage example.
///
/// This macro can optionally take a `raw_return` argument specifying that the return type, which
/// must be a [`Value`], will be returned as is, and not go through serialization.
///
/// Additionally, function parameters can also use the following attributes:
///
/// * `#[raw]` with a parameter of type [`Value`]: The parameter will be passed as
///   is and not go through deserialization.
/// * `#[try_from_ref]`: Instead of regular deserialization, `TryFrom::try_from(&Value)` will be
///   used.
///
///   Implementing the `TryFrom` trait accordingly can make using blessed references more
///   convenient, but at the cost of hiding underlying `unsafe` code.
pub use perlmod_macro::export;
