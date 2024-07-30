/// The fhe following requires rust 1.42 to work, as custom attributes on inline modules has only
/// been stabilized then.
mod pkg142;

/*
#[cfg(feature = "rustmuchlater")]
/// The following is what we ideally want to reach with future rust versions. It is technically
/// possible on nightly with #![feature(custom_inner_attributes)]
mod pkginline;
*/

/// A test for blessed values.
mod bless;

/// Tests for `Option`.
mod option;

/// Tests for magic based blessed objects.
mod magic;
