use anyhow::Error;

use proc_macro2::{Ident, TokenStream, Span};

use quote::quote;

use crate::attribs::FunctionAttrs;

pub struct XSub {
    pub rust_name: Ident,
    pub xs_name: Ident,
    pub tokens: TokenStream,
}

pub fn handle_function(attr: FunctionAttrs, func: syn::ItemFn) -> Result<XSub, Error> {
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

    let too_many_args_error = syn::LitStr::new(
        &format!("too many parameters for function '{}', (expected {})", name, sig.inputs.len()),
        Span::call_site(),
    );

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

        #[inline(never)]
        fn #impl_xs_name(
            _cv: &::perlmod::ffi::CV,
        ) -> Result<*mut ::perlmod::ffi::SV, *mut ::perlmod::ffi::SV> {
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
