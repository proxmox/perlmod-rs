use anyhow::Error;

use proc_macro2::{Ident, Span, TokenStream};

use quote::quote;

use crate::attribs::FunctionAttrs;

pub struct XSub {
    pub rust_name: Ident,
    pub perl_name: Option<Ident>,
    pub xs_name: Ident,
    pub tokens: TokenStream,
}

pub fn handle_function(
    attr: FunctionAttrs,
    mut func: syn::ItemFn,
    mangled_package_name: Option<&str>,
) -> Result<XSub, Error> {
    if !func.sig.generics.params.is_empty() {
        bail!(&func.sig.generics => "generic functions cannot be exported as xsubs");
    }

    if func.sig.asyncness.is_some() {
        bail!(&func.sig.asyncness => "async fns cannot be exported as xsubs");
    }

    let name = func.sig.ident.clone();
    let xs_name = attr.xs_name.unwrap_or_else(|| match mangled_package_name {
        None => Ident::new(&format!("xs_{}", name), name.span()),
        Some(prefix) => Ident::new(&format!("xs_{}_{}", prefix, name), name.span()),
    });
    let impl_xs_name = Ident::new(&format!("impl_xs_{}", name), name.span());

    let mut extract_arguments = TokenStream::new();
    let mut deserialized_arguments = TokenStream::new();
    let mut passed_arguments = TokenStream::new();
    for arg in &mut func.sig.inputs {
        let mut raw_arg = false;

        let pat_ty = match arg {
            syn::FnArg::Receiver(_) => bail!(arg => "cannot export self-taking methods as xsubs"),
            syn::FnArg::Typed(ref mut pt) => {
                pt.attrs.retain(|attr| {
                    if attr.path.is_ident("raw") {
                        raw_arg = true;
                        false
                    } else {
                        true
                    }
                });
                &*pt
            }
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

        if raw_arg {
            deserialized_arguments.extend(quote! {
                let #deserialized_name = #extracted_name;
            });
        } else {
            deserialized_arguments.extend(quote! {
                let #deserialized_name: #arg_type =
                    match ::perlmod::from_ref_value(&#extracted_name) {
                        Ok(data) => data,
                        Err(err) => {
                            return Err(::perlmod::Value::new_string(&err.to_string())
                                .into_mortal()
                                .into_raw());
                        }
                    };
            });
        }

        if passed_arguments.is_empty() {
            passed_arguments.extend(quote! { #deserialized_name });
        } else {
            passed_arguments.extend(quote! {, #deserialized_name });
        }
    }

    let has_return_value = match &func.sig.output {
        syn::ReturnType::Default => false,
        syn::ReturnType::Type(_arrow, ty) => match &**ty {
            syn::Type::Tuple(tuple) => !tuple.elems.is_empty(),
            _ => true,
        },
    };

    let too_many_args_error = syn::LitStr::new(
        &format!(
            "too many parameters for function '{}', (expected {})",
            name,
            func.sig.inputs.len()
        ),
        Span::call_site(),
    );

    let mut handle_return;
    let return_type;
    let wrapper_func;
    if has_return_value {
        return_type = quote! { *mut ::perlmod::ffi::SV };

        handle_return = quote! {
            let result = match #name(#passed_arguments) {
                Ok(output) => output,
                Err(err) => {
                    return Err(::perlmod::Value::new_string(&err.to_string())
                        .into_mortal()
                        .into_raw());
                }
            };
        };

        if attr.raw_return {
            handle_return.extend(quote! {
                Ok(result.into_mortal().into_raw())
            });
        } else {
            handle_return.extend(quote! {
                match ::perlmod::to_value(&result) {
                    Ok(value) => Ok(value.into_mortal().into_raw()),
                    Err(err) => Err(::perlmod::Value::new_string(&err.to_string())
                        .into_mortal()
                        .into_raw()),
                }
            });
        };

        wrapper_func = quote! {
            #[no_mangle]
            pub extern "C" fn #xs_name(cv: &::perlmod::ffi::CV) {
                unsafe {
                    match #impl_xs_name(cv) {
                        Ok(sv) => ::perlmod::ffi::stack_push_raw(sv),
                        Err(sv) => ::perlmod::ffi::croak(sv),
                    }
                }
            }
        };
    } else {
        return_type = quote! { () };

        if attr.raw_return {
            bail!(&attr.raw_return => "raw_return attribute is illegal without a return value");
        }

        handle_return = quote! {
            #name(#passed_arguments);

            Ok(())
        };

        wrapper_func = quote! {
            #[no_mangle]
            pub extern "C" fn #xs_name(cv: &::perlmod::ffi::CV) {
                unsafe {
                    match #impl_xs_name(cv) {
                        Ok(()) => (),
                        Err(sv) => ::perlmod::ffi::croak(sv),
                    }
                }
            }
        };
    }

    let tokens = quote! {
        #func

        #wrapper_func

        #[inline(never)]
        #[allow(non_snake_case)]
        fn #impl_xs_name(
            _cv: &::perlmod::ffi::CV,
        ) -> Result<#return_type, *mut ::perlmod::ffi::SV> {
            let argmark = unsafe { ::perlmod::ffi::pop_arg_mark() };
            let mut args = argmark.iter();

            #extract_arguments

            if args.next().is_some() {
                return Err(::perlmod::Value::new_string(#too_many_args_error)
                    .into_mortal()
                    .into_raw());
            }

            drop(args);

            #deserialized_arguments

            unsafe {
                argmark.set_stack();
            }

            #handle_return
        }
    };

    Ok(XSub {
        rust_name: name,
        perl_name: attr.perl_name,
        xs_name,
        tokens,
    })
}
