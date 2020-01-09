pub(crate) mod error;

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
pub use perlmod_macro::package;
