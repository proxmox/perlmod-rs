use proc_macro2::TokenStream;

use quote::quote;
use syn::punctuated::Punctuated;
use syn::{Error, Meta, Token};

use crate::attribs::FunctionAttrs;
use crate::package::Package;

pub fn handle_module(
    attr: Punctuated<Meta, Token![,]>,
    mut module: syn::ItemMod,
) -> Result<TokenStream, Error> {
    let mut package = Package::with_attrs(attr)?;
    let mangled_package_name = package.mangle_package_name();

    if let Some((_brace, ref mut items)) = module.content {
        for item in items.iter_mut() {
            match core::mem::replace(item, syn::Item::Verbatim(TokenStream::new())) {
                syn::Item::Fn(mut func) => {
                    let mut attribs = None;
                    for attr in std::mem::take(&mut func.attrs) {
                        if !attr.path().is_ident("export") {
                            // retain the attribute
                            func.attrs.push(attr);
                            continue;
                        }
                        if attribs.is_some() {
                            error!(attr => "multiple 'export' attributes not allowed");
                            continue;
                        }

                        let args = match attr.meta {
                            Meta::Path(_) => Default::default(),
                            Meta::List(list) => list.parse_args_with(
                                Punctuated::<syn::Meta, Token![,]>::parse_terminated,
                            )?,
                            _ => {
                                error!(attr => "invalid 'export' attribute syntax");
                                continue;
                            }
                        };

                        attribs = Some(FunctionAttrs::try_from(args)?);
                    }

                    // if we removed an #[export] macro this is an exported function:
                    if let Some(attribs) = attribs {
                        let func = crate::function::handle_function(
                            attribs,
                            func,
                            Some(&mangled_package_name),
                            false,
                        )?;
                        *item = syn::Item::Verbatim(func.tokens);

                        package.export_named(
                            func.rust_name,
                            func.perl_name,
                            func.xs_name,
                            func.prototype,
                        );
                    } else {
                        *item = syn::Item::Fn(func);
                    }
                }
                other => *item = other,
            }
        }

        items.push(syn::Item::Verbatim(package.bootstrap_function()));
    }

    if package.attrs.write == Some(true)
        || package.attrs.file_name.is_some()
        || std::env::var("PERLMOD_WRITE_PACKAGES").ok().as_deref() == Some("1")
    {
        package.write()?;
    }

    Ok(quote! { #module })
}
