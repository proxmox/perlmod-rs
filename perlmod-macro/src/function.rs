use proc_macro2::{Ident, Span, TokenStream};

use quote::quote;
use syn::Error;

use crate::attribs::FunctionAttrs;

pub struct XSub {
    pub rust_name: Ident,
    pub perl_name: Option<Ident>,
    pub xs_name: Ident,
    pub tokens: TokenStream,
}

#[derive(Default)]
struct ArgumentAttrs {
    raw: bool,
    try_from_ref: bool,
}

impl ArgumentAttrs {
    fn handle_path(&mut self, path: &syn::Path) -> bool {
        if path.is_ident("raw") {
            self.raw = true;
        } else if path.is_ident("try_from_ref") {
            self.try_from_ref = true;
        } else {
            return false;
        }

        true
    }

    fn validate(&self, span: Span) -> Result<(), Error> {
        if self.raw && self.try_from_ref {
            bail!(
                span,
                "`raw` and `try_from_ref` attributes are mutually exclusive"
            );
        }
        Ok(())
    }
}

enum Return {
    /// Return nothing. (This is different from returning an implicit undef!)
    None(bool),

    /// Return a single element.
    Single(bool),

    /// We support tuple return types. They act like "list" return types in perl.
    Tuple(bool, usize),
}

pub fn handle_function(
    attr: FunctionAttrs,
    mut func: syn::ItemFn,
    mangled_package_name: Option<&str>,
    export_public: bool,
) -> Result<XSub, Error> {
    if !func.sig.generics.params.is_empty() {
        bail!(&func.sig.generics => "generic functions cannot be exported as xsubs");
    }

    if func.sig.asyncness.is_some() {
        bail!(&func.sig.asyncness => "async fns cannot be exported as xsubs");
    }

    let name = func.sig.ident.clone();
    let xs_name = attr
        .xs_name
        .clone()
        .unwrap_or_else(|| match mangled_package_name {
            None => Ident::new(&format!("xs_{}", name), name.span()),
            Some(prefix) => Ident::new(&format!("xs_{}_{}", prefix, name), name.span()),
        });
    let impl_xs_name = Ident::new(&format!("impl_xs_{}", name), name.span());

    let mut extract_arguments = TokenStream::new();
    let mut deserialized_arguments = TokenStream::new();
    let mut passed_arguments = TokenStream::new();
    for arg in &mut func.sig.inputs {
        let mut argument_attrs = ArgumentAttrs::default();

        let pat_ty = match arg {
            syn::FnArg::Receiver(_) => bail!(arg => "cannot export self-taking methods as xsubs"),
            syn::FnArg::Typed(ref mut pt) => {
                pt.attrs
                    .retain(|attr| !argument_attrs.handle_path(&attr.path));
                use syn::spanned::Spanned;
                argument_attrs.validate(pt.span())?;
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

        let missing_message = syn::LitStr::new(
            &format!("missing required parameter: '{}'\n", arg_name),
            arg_name.span(),
        );

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

        if argument_attrs.raw {
            deserialized_arguments.extend(quote! {
                let #deserialized_name = #extracted_name;
            });
        } else if argument_attrs.try_from_ref {
            deserialized_arguments.extend(quote! {
                let #deserialized_name: #arg_type =
                    match ::std::convert::TryFrom::try_from(&#extracted_name) {
                        Ok(arg) => arg,
                        Err(err) => {
                            return Err(::perlmod::Value::new_string(&format!("{}\n", err))
                                .into_mortal()
                                .into_raw());
                        }
                    };
            });
        } else {
            deserialized_arguments.extend(quote! {
                let #deserialized_name: #arg_type =
                    match ::perlmod::from_ref_value(&#extracted_name) {
                        Ok(data) => data,
                        Err(err) => {
                            return Err(::perlmod::Value::new_string(&format!("{}\n", err))
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
        syn::ReturnType::Default => Return::None(false),
        syn::ReturnType::Type(_arrow, ty) => match get_result_type(&**ty) {
            (syn::Type::Tuple(tuple), result) if tuple.elems.is_empty() => Return::None(result),
            (syn::Type::Tuple(tuple), result) => Return::Tuple(result, tuple.elems.len()),
            (_, result) => Return::Single(result),
        },
    };

    let too_many_args_error = syn::LitStr::new(
        &format!(
            "too many parameters for function '{}', (expected {})\n",
            name,
            func.sig.inputs.len()
        ),
        Span::call_site(),
    );

    let ReturnHandling {
        return_type,
        handle_return,
        wrapper_func,
    } = handle_return_kind(
        &attr,
        has_return_value,
        &name,
        &xs_name,
        &impl_xs_name,
        passed_arguments,
        export_public,
    )?;

    let tokens = quote! {
        #func

        #wrapper_func

        #[inline(never)]
        #[allow(non_snake_case)]
        fn #impl_xs_name() -> Result<#return_type, *mut ::perlmod::ffi::SV> {
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

struct ReturnHandling {
    return_type: TokenStream,
    handle_return: TokenStream,
    wrapper_func: TokenStream,
}

fn handle_return_kind(
    attr: &FunctionAttrs,
    ret: Return,
    name: &Ident,
    xs_name: &Ident,
    impl_xs_name: &Ident,
    passed_arguments: TokenStream,
    export_public: bool,
) -> Result<ReturnHandling, Error> {
    let return_type;
    let mut handle_return;
    let wrapper_func;

    let pthx = crate::pthx_param();
    match ret {
        Return::None(result) => {
            return_type = quote! { () };

            if attr.raw_return {
                bail!(&attr.raw_return => "raw_return attribute is illegal without a return value");
            }

            if result {
                handle_return = quote! {
                    match #name(#passed_arguments) {
                        Ok(()) => (),
                        Err(err) => {
                            return Err(::perlmod::Value::new_string(&format!("{}\n", err))
                                .into_mortal()
                                .into_raw());
                        }
                    }

                    Ok(())
                };
            } else {
                handle_return = quote! {
                    #name(#passed_arguments);

                    Ok(())
                };
            }

            let vis = if export_public {
                quote! { #[no_mangle] pub }
            } else {
                quote! { #[allow(non_snake_case)] }
            };

            wrapper_func = quote! {
                #[doc(hidden)]
                #vis extern "C" fn #xs_name(#pthx _cv: &::perlmod::ffi::CV) {
                    unsafe {
                        match #impl_xs_name() {
                            Ok(()) => (),
                            Err(sv) => ::perlmod::ffi::croak(sv),
                        }
                    }
                }
            };
        }
        Return::Single(result) => {
            return_type = quote! { *mut ::perlmod::ffi::SV };

            if result {
                handle_return = quote! {
                    let result = match #name(#passed_arguments) {
                        Ok(output) => output,
                        Err(err) => {
                            return Err(::perlmod::Value::new_string(&format!("{}\n", err))
                                .into_mortal()
                                .into_raw());
                        }
                    };
                };
            } else {
                handle_return = quote! {
                    let result = #name(#passed_arguments);
                };
            }

            if attr.raw_return {
                handle_return.extend(quote! {
                    Ok(result.into_mortal().into_raw())
                });
            } else {
                handle_return.extend(quote! {
                    match ::perlmod::to_value(&result) {
                        Ok(value) => Ok(value.into_mortal().into_raw()),
                        Err(err) => Err(::perlmod::Value::new_string(&format!("{}\n", err))
                            .into_mortal()
                            .into_raw()),
                    }
                });
            };

            wrapper_func = quote! {
                #[no_mangle]
                #[doc(hidden)]
                pub extern "C" fn #xs_name(#pthx _cv: &::perlmod::ffi::CV) {
                    unsafe {
                        match #impl_xs_name() {
                            Ok(sv) => ::perlmod::ffi::stack_push_raw(sv),
                            Err(sv) => ::perlmod::ffi::croak(sv),
                        }
                    }
                }
            };
        }
        Return::Tuple(result, count) => {
            return_type = {
                let mut rt = TokenStream::new();
                for _ in 0..count {
                    rt.extend(quote! { *mut ::perlmod::ffi::SV, });
                }
                quote! { (#rt) }
            };

            if result {
                handle_return = quote! {
                    let result = match #name(#passed_arguments) {
                        Ok(output) => output,
                        Err(err) => {
                            return Err(::perlmod::Value::new_string(&format!("{}\n", err))
                                .into_mortal()
                                .into_raw());
                        }
                    };
                };
            } else {
                handle_return = quote! {
                    let result = match #name(#passed_arguments);
                };
            }

            let mut rt = TokenStream::new();
            if attr.raw_return {
                for i in 0..count {
                    let i = simple_usize(i, Span::call_site());
                    rt.extend(quote! { (result.#i).into_mortal().into_raw(), });
                }
            } else {
                for i in 0..count {
                    let i = simple_usize(i, Span::call_site());
                    rt.extend(quote! {
                        match ::perlmod::to_value(&result.#i) {
                            Ok(value) => value.into_mortal().into_raw(),
                            Err(err) => return
                                Err(::perlmod::Value::new_string(&format!("{}\n", err))
                                    .into_mortal()
                                    .into_raw()),
                        },
                    });
                }
            }
            handle_return.extend(quote! {
                Ok((#rt))
            });
            drop(rt);

            let icount = simple_usize(count, Span::call_site());
            let sp_offset = simple_usize(count - 1, Span::call_site());
            let mut push = quote! {
                ::perlmod::ffi::RSPL_stack_resize_by(#icount);
                let mut sp = ::perlmod::ffi::RSPL_stack_sp().sub(#sp_offset);
                *sp = sv.0;
            };

            for i in 1..count {
                let i = simple_usize(i, Span::call_site());
                push.extend(quote! {
                    sp = sp.add(1);
                    *sp = sv.#i;
                });
            }
            //let mut push = TokenStream::new();
            //for i in 0..count {
            //    let i = simple_usize(i, Span::call_site());
            //    push.extend(quote! {
            //        ::perlmod::ffi::stack_push_raw(sv.#i);
            //    });
            //}

            wrapper_func = quote! {
                #[no_mangle]
                #[doc(hidden)]
                pub extern "C" fn #xs_name(#pthx _cv: &::perlmod::ffi::CV) {
                    unsafe {
                        match #impl_xs_name() {
                            Ok(sv) => { #push },
                            Err(sv) => ::perlmod::ffi::croak(sv),
                        }
                    }
                }
            };
        }
    }

    Ok(ReturnHandling {
        return_type,
        handle_return,
        wrapper_func,
    })
}

/// Note that we cannot handle renamed imports at all here...
pub fn is_result_type(ty: &syn::Type) -> Option<&syn::Type> {
    if let syn::Type::Path(p) = ty {
        if p.qself.is_some() {
            return None;
        }
        let segs = &p.path.segments;
        let is_result = match segs.len() {
            1 => segs.last().unwrap().ident == "Result",
            2 => segs.first().unwrap().ident == "std" && segs.last().unwrap().ident == "Result",
            _ => false,
        };
        if !is_result {
            return None;
        }

        if let syn::PathArguments::AngleBracketed(generic) = &segs.last().unwrap().arguments {
            // We allow aliased Result types with an implicit Error:
            if generic.args.len() != 1 && generic.args.len() != 2 {
                return None;
            }

            if let syn::GenericArgument::Type(ty) = generic.args.first().unwrap() {
                return Some(ty);
            }
        }
    }
    None
}

/// If the type is a Result type, return the contained Ok type, otherwise return the type itself.
/// Also return whether or not it actually was a Result.
pub fn get_result_type(ty: &syn::Type) -> (&syn::Type, bool) {
    match is_result_type(ty) {
        Some(ty) => (ty, true),
        None => (ty, false),
    }
}

/// Get a non-suffixed integer from an usize.
fn simple_usize(i: usize, span: Span) -> syn::LitInt {
    syn::LitInt::new(&format!("{}", i), span)
}
