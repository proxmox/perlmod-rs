//! Proc macros and attributes for the `perlmod` crate.
//!
//! See the `perlmod` crate's documentation for deatails.

extern crate proc_macro;
extern crate proc_macro2;

use std::convert::TryFrom;

use anyhow::Error;

use proc_macro::TokenStream as TokenStream_1;
use proc_macro2::TokenStream;

use syn::parse_macro_input;
use syn::AttributeArgs;

macro_rules! format_err {
    ($span:expr => $($msg:tt)*) => { syn::Error::new_spanned($span, format!($($msg)*)) };
    ($span:expr, $($msg:tt)*) => { syn::Error::new($span, format!($($msg)*)) };
}

macro_rules! bail {
    ($span:expr => $($msg:tt)*) => { return Err(format_err!($span => $($msg)*).into()) };
    ($span:expr, $($msg:tt)*) => { return Err(format_err!($span, $($msg)*).into()) };
}

mod attribs;
mod function;
mod module;
mod package;

fn handle_error(mut item: TokenStream, data: Result<TokenStream, Error>) -> TokenStream {
    match data {
        Ok(output) => output,
        Err(err) => match err.downcast::<syn::Error>() {
            Ok(err) => {
                item.extend(err.to_compile_error());
                item
            }
            Err(err) => panic!("error in api/router macro: {}", err),
        },
    }
}

/// Attribute for making a perl "package" out of a rust module.
///
/// This can be used on an inline module (rust version 1.42 or later), and hopefully in the future
/// as an inline attribute (`#![package(name = "Some::Package")]`).
///
/// ```
/// // 'lib' and 'file' are optional. We use 'file' here to prevent doc tests from writing out the
/// // file.
/// //
/// // 'name', 'lib' and 'file' expand environment variables such as `${CARGO_PKG_NAME}`
/// #[perlmod::package(name = "RSPM::Foo", lib = "perlmod_test", file = "/dev/null")]
/// mod an_unused_name {
///     use anyhow::{bail, Error};
///
///     // This function can be used like `RSPM::Foo::foo(1, 2);` in perl.
///     #[export]
///     fn foo(a: u32, b: u32) -> Result<u32, Error> {
///         if a == 42 {
///             bail!("dying on magic number");
///         }
///
///         Ok(a + b)
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn package(attr: TokenStream_1, item: TokenStream_1) -> TokenStream_1 {
    let attr = parse_macro_input!(attr as AttributeArgs);
    let item: TokenStream = item.into();
    handle_error(item.clone(), perlmod_impl(attr, item)).into()
}

/// Attribute to export a function so that it can be installed as an `xsub` in perl. See the
/// [`package!`](macro@package) macro for a usage example.
#[proc_macro_attribute]
pub fn export(attr: TokenStream_1, item: TokenStream_1) -> TokenStream_1 {
    let attr = parse_macro_input!(attr as AttributeArgs);
    let item: TokenStream = item.into();
    handle_error(item.clone(), export_impl(attr, item)).into()
}

fn perlmod_impl(attr: AttributeArgs, item: TokenStream) -> Result<TokenStream, Error> {
    let item: syn::Item = syn::parse2(item)?;

    match item {
        syn::Item::Fn(func) => bail!(func => "did you mean to use the 'export' macro?"),
        syn::Item::Mod(module) => module::handle_module(attr, module),
        _ => bail!(item => "expected module or function"),
    }
}

fn export_impl(attr: AttributeArgs, item: TokenStream) -> Result<TokenStream, Error> {
    let func: syn::ItemFn = syn::parse2(item)?;

    let attr = attribs::FunctionAttrs::try_from(attr)?;
    let func = function::handle_function(attr, func, None)?;
    Ok(func.tokens)
}
