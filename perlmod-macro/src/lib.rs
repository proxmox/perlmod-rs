extern crate proc_macro;
extern crate proc_macro2;

use std::convert::TryFrom;
use std::iter::IntoIterator;

use failure::Error;

use proc_macro::TokenStream as TokenStream_1;
use proc_macro2::{Ident, TokenStream};

use quote::quote;
use syn::parse::Parser;
use syn::parse_macro_input;
use syn::punctuated::Punctuated;
use syn::{AttributeArgs, Token};

macro_rules! format_err {
    ($span:expr => $($msg:tt)*) => { syn::Error::new_spanned($span, format!($($msg)*)) };
    ($span:expr, $($msg:tt)*) => { syn::Error::new($span, format!($($msg)*)) };
}

macro_rules! bail {
    ($span:expr => $($msg:tt)*) => { return Err(format_err!($span => $($msg)*).into()) };
    ($span:expr, $($msg:tt)*) => { return Err(format_err!($span, $($msg)*).into()) };
}

mod attribs;

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

struct XSub {
    rust_name: Ident,
    xs_name: Ident,
    tokens: TokenStream,
}

/// Macro for starting a perl "package".
#[proc_macro_attribute]
pub fn package(attr: TokenStream_1, item: TokenStream_1) -> TokenStream_1 {
    let attr = parse_macro_input!(attr as AttributeArgs);
    let item: TokenStream = item.into();
    handle_error(item.clone(), perlmod_impl(attr, item)).into()
}

/// Macro to generate an exported xsub for a function.
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
        syn::Item::Mod(module) => handle_module(attr, module),
        _ => bail!(item => "expected module or function"),
    }
}

fn export_impl(attr: AttributeArgs, item: TokenStream) -> Result<TokenStream, Error> {
    let func: syn::ItemFn = syn::parse2(item)?;

    let attr = attribs::FunctionAttrs::try_from(attr)?;
    let func = handle_function(attr, func)?;
    Ok(func.tokens)
}

fn handle_function(attr: attribs::FunctionAttrs, func: syn::ItemFn) -> Result<XSub, Error> {
    //let vis = core::mem::replace(&mut func.vis, syn::Visibility::Inherited);
    //if let syn::Visibility::Public(_) = vis {
    //    // ok
    //} else {
    //    bail!(func.sig.fn_token => "only public functions can be exported as xsubs");
    //}

    let sig = &func.sig;
    if !sig.generics.params.is_empty() {
        bail!(&sig.generics => "generic functions cannot be exported as xsubs");
    }

    if sig.asyncness.is_some() {
        bail!(&sig.asyncness => "async fns cannot be exported as xsubs");
    }

    let name = &sig.ident;
    let xs_name = attr
        .xs_name
        .unwrap_or_else(|| Ident::new(&format!("xs_{}", name), name.span()));
    let impl_xs_name = Ident::new(&format!("impl_xs_{}", name), name.span());

    let mut extract_arguments = TokenStream::new();
    let mut deserialized_arguments = TokenStream::new();
    let mut passed_arguments = TokenStream::new();
    for arg in &sig.inputs {
        let pat_ty = match arg {
            syn::FnArg::Receiver(_) => bail!(arg => "cannot export self-taking methods as xsubs"),
            syn::FnArg::Typed(pt) => pt,
        };

        let arg_name = match &*pat_ty.pat {
            syn::Pat::Ident(ident) => {
                if ident.by_ref.is_some() {
                    bail!(ident => "xsub does not support by-ref parameters");
                }
                if ident.subpat.is_some() {
                    bail!(ident => "xsub does not support sub-patterns on parameters");
                }
                &ident.ident
            }
            _ => bail!(&pat_ty.pat => "xsub does not support this kind of parameter"),
        };

        let arg_type = &*pat_ty.ty;

        let extracted_name = Ident::new(&format!("extracted_arg_{}", arg_name), arg_name.span());
        let deserialized_name =
            Ident::new(&format!("deserialized_arg_{}", arg_name), arg_name.span());

        let missing_message = syn::LitStr::new("missing required parameter: '{}'", arg_name.span());

        extract_arguments.extend(quote! {
            let #extracted_name: ::perlmod::Value = match args.next() {
                Some(arg) => ::perlmod::Value::from(arg),
                None => {
                    return Err(::perlmod::Value::new_string(#missing_message)
                        .into_mortal()
                        .into_raw());
                }
            };
        });

        deserialized_arguments.extend(quote! {
            let #deserialized_name: #arg_type = match ::perlmod::from_value(#extracted_name) {
                Ok(data) => data,
                Err(err) => {
                    return Err(::perlmod::Value::new_string(&err.to_string())
                        .into_mortal()
                        .into_raw());
                }
            };
        });

        if passed_arguments.is_empty() {
            passed_arguments.extend(quote! { #deserialized_name });
        } else {
            passed_arguments.extend(quote! {, #deserialized_name });
        }
    }

    let tokens = quote! {
        #func

        #[no_mangle]
        pub extern "C" fn #xs_name(cv: &::perlmod::ffi::CV) {
            unsafe {
                match #impl_xs_name(cv) {
                    Ok(sv) => ::perlmod::ffi::stack_push_raw(sv),
                    Err(sv) => ::perlmod::ffi::croak(sv),
                }
            }
        }

        fn #impl_xs_name(
            _cv: &::perlmod::ffi::CV,
        ) -> Result<*mut ::perlmod::ffi::SV, *mut ::perlmod::ffi::SV> {
            let argmark = unsafe { ::perlmod::ffi::pop_arg_mark() };
            let mut args = argmark.iter();

            #extract_arguments

            drop(args);

            #deserialized_arguments

            unsafe {
                argmark.set_stack();
            }

            let result = match #name(#passed_arguments) {
                Ok(output) => output,
                Err(err) => {
                    return Err(::perlmod::Value::new_string(&err.to_string())
                        .into_mortal()
                        .into_raw());
                }
            };

            match ::perlmod::to_value(&result) {
                Ok(value) => Ok(value.into_mortal().into_raw()),
                Err(err) => Err(::perlmod::Value::new_string(&err.to_string())
                    .into_mortal()
                    .into_raw()),
            }
        }
    };

    Ok(XSub {
        rust_name: name.to_owned(),
        xs_name,
        tokens,
    })
}

const LIB_NAME_DEFAULT: &str = r#"($pkg =~ /(?:^|::)([^:]+)$/)"#;

const MODULE_HEAD: &str = r#"
use strict;
use warnings;
use DynaLoader ();

my $LIB;

sub __load_shared_lib {
    return if $LIB;

    my ($pkg) = @_;

    my $auto_path = ($pkg =~ s!::!/!gr);
    my ($mod_name) = {{LIB_NAME}};

    my @dirs = (map "-L$_/auto/$auto_path", @INC);
    my (@mod_files) = DynaLoader::dl_findfile(@dirs, '-L./target/debug', $mod_name);
    die "failed to locate shared library for '$pkg' (lib${mod_name}.so)\n" if !@mod_files;

    $LIB = DynaLoader::dl_load_file($mod_files[0])
        or die "failed to load library '$mod_files[0]'\n";
}

sub newXS {
    my ($perl_func_name, $full_symbol_name, $filename) = @_;

    my $sym  = DynaLoader::dl_find_symbol($LIB, $full_symbol_name);
    die "failed to locate '$full_symbol_name'\n" if !defined $sym;
    DynaLoader::dl_install_xsub($perl_func_name, $sym, $filename);
}

BEGIN {
    __load_shared_lib(__PACKAGE__);
"#;

const MODULE_TAIL: &str = "}\n";

fn handle_module(attr: AttributeArgs, mut module: syn::ItemMod) -> Result<TokenStream, Error> {
    let args = attribs::ModuleAttrs::try_from(attr)?;

    let mut module_source = format!("package {};\n{}", args.package_name, MODULE_HEAD);

    if let Some((_brace, ref mut items)) = module.content {
        for item in items.iter_mut() {
            match core::mem::replace(item, syn::Item::Verbatim(TokenStream::new())) {
                syn::Item::Fn(mut func) => {
                    let mut attribs = None;
                    for attr in std::mem::replace(&mut func.attrs, Default::default()) {
                        if attr.path.is_ident("export") {
                            if attribs.is_some() {
                                bail!(attr => "multiple 'export' attributes not allowed");
                            }

                            let args: AttributeArgs =
                                Punctuated::<syn::NestedMeta, Token![,]>::parse_terminated
                                    .parse2(attr.tokens)?
                                    .into_iter()
                                    .collect();

                            attribs = Some(attribs::FunctionAttrs::try_from(args)?);
                        } else {
                            // retain the attribute
                            func.attrs.push(attr);
                        }
                    }

                    // if we removed an #[export] macro this is an exported function:
                    if let Some(attribs) = attribs {
                        let func = handle_function(attribs, func)?;
                        *item = syn::Item::Verbatim(func.tokens);
                        module_source = format!(
                            "{}    newXS('{}', '{}', 'src/FIXME.rs');\n",
                            module_source, func.rust_name, func.xs_name,
                        );
                    } else {
                        *item = syn::Item::Fn(func);
                    }
                }
                other => *item = other,
            }
        }
    }

    module_source.push_str(MODULE_TAIL);

    if let Some(lib) = args.lib_name {
        module_source = module_source.replace("{{LIB_NAME}}", &format!("('{}')", lib));
    } else {
        module_source = module_source.replace("{{LIB_NAME}}", LIB_NAME_DEFAULT);
    }

    let path = std::path::Path::new(&args.file_name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, module_source.as_bytes())?;

    Ok(quote! { #module })
}
