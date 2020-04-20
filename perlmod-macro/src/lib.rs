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
/// [`make_package!`] macro for a usage example.
#[proc_macro_attribute]
pub fn export(attr: TokenStream_1, item: TokenStream_1) -> TokenStream_1 {
    let attr = parse_macro_input!(attr as AttributeArgs);
    let item: TokenStream = item.into();
    handle_error(item.clone(), export_impl(attr, item)).into()
}

/// Proc macro to create a perl package file for rust functions.
///
/// This macro will write a perl package/module file into cargo's working directory. (Typically the
/// manifest directory.)
///
/// This macro exists mostly for backward compatibility. When using rustc 1.42 or above, a more
/// readable and less repetitive code will be produced with the [`package`](module@crate::package)
/// attribute instead.
///
/// This macro always has to be used in conjunction with the [`export!]` macro, like this:
///
/// ```
/// # mod testmod {
/// use anyhow::{bail, Error};
/// use perlmod::export;
///
/// #[export]
/// fn sum_except_42(a: u32, b: u32) -> Result<u32, Error> {
///     if a == 42 {
///         // Errors 'die' in perl, so newlines at the end of error messages make a difference!
///         bail!("dying on magic number\n");
///     }
///
///     Ok(a + b)
/// }
///
/// #[export(name = "xs_sub_name")]
/// fn double(a: u32) -> Result<u32, Error> {
///     Ok(2 * a)
/// }
///
/// perlmod::make_package! {
///     // First we need to specify the package, similar to perl's syntax:
///     package "RSPM::DocTest1";
///
///     // The library name is usually derived from the crate name in Cargo.toml automatically.
///     // So this line is optional:
///     lib "perlmod_test";
///
///     // An optional output file name can be specified as follows:
///     // (we use this here to prevent doc tests from creating files...)
///     file "/dev/null";
///
///     // The list of xsubs we want to export:
///     subs {
///         // When only providing the name, default naming convention will be used:
///         // This is used like: `RSPM::DocTest1::sum_except_42(4, 5);` in perl.
///         sum_except_42,
///         // If we used an explicit export name, we need to also explicitly export the renamed
///         // function here:
///         // This is used like: `RSPM::DocTest1::double_the_number(5);` in perl.
///         xs_sub_name as double_the_number,
///     }
/// }
/// # }
/// ```
#[proc_macro]
pub fn make_package(item: TokenStream_1) -> TokenStream_1 {
    let item: TokenStream = item.into();
    handle_error(item.clone(), make_package_impl(item)).into()
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
    let func = function::handle_function(attr, func)?;
    Ok(func.tokens)
}

fn make_package_impl(item: TokenStream) -> Result<TokenStream, Error> {
    let pkg: package::Package = syn::parse2(item)?;
    pkg.write()?;
    Ok(TokenStream::new())
}
