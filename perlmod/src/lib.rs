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
pub use de::from_value;
#[doc(inline)]
pub use ser::to_value;

pub mod scalar;
#[doc(inline)]
pub use scalar::{Mortal, Scalar};

pub mod array;
#[doc(inline)]
pub use array::Array;

pub mod hash;
#[doc(inline)]
pub use hash::Hash;

pub mod value;
#[doc(inline)]
pub use value::Value;

#[cfg(feature = "exporter")]
#[doc(inline)]
pub use perlmod_macro::{export, package};
