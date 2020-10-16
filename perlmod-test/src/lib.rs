/// The fhe following requires rust 1.42 to work, as custom attributes on inline modules has only
/// been stabilized then.
mod pkg142;

#[cfg(feature = "rustmuchlater")]
/// The following is what we ideally want to reach with future rust versions. It is technically
/// possible on nightly with #![feature(custom_inner_attributes)]
mod pkginline;

/// This is possible on stable rust with some 1.3x already.
mod pkgstable;
