use proc_macro2::{Ident, Span, TokenStream};

use quote::quote;
use syn::spanned::Spanned;
use syn::Error;

use crate::attribs::FunctionAttrs;

pub struct XSub {
    pub rust_name: Ident,
    pub perl_name: Option<Ident>,
    pub xs_name: Ident,
    pub tokens: TokenStream,
    pub prototype: Option<String>,
}

#[derive(Default)]
struct ArgumentAttrs {
    /// This is the `CV` pointer.
    cv: Option<Span>,

    /// Skip the deserializer for this argument.
    raw: bool,

    /// Call `TryFrom<&Value>::try_from` for this argument instead of deserializing it.
    try_from_ref: bool,
}

impl ArgumentAttrs {
    fn handle_path(&mut self, path: &syn::Path) -> bool {
        if path.is_ident("raw") {
            self.raw = true;
        } else if path.is_ident("try_from_ref") {
            self.try_from_ref = true;
        } else if path.is_ident("cv") {
            self.cv = Some(path.span());
        } else {
            return false;
        }

        true
    }

    fn validate(&self, span: Span) -> Result<(), Error> {
        if self.raw as usize + self.try_from_ref as usize + self.cv.is_some() as usize > 1 {
            bail!(
                span,
                "`raw` and `try_from_ref` and `cv` attributes are mutually exclusive"
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
    let export_public = export_public.then(|| &func.vis);
    let xs_name = attr
        .xs_name
        .clone()
        .unwrap_or_else(|| match mangled_package_name {
            None => Ident::new(&format!("xs_{}", name), name.span()),
            Some(prefix) => Ident::new(&format!("xs_{}_{}", prefix, name), name.span()),
        });
    let impl_xs_name = Ident::new(&format!("impl_xs_{}", name), name.span());

    let mut trailing_options = 0;
    let mut extract_arguments = TokenStream::new();
    let mut deserialized_arguments = TokenStream::new();
    let mut passed_arguments = TokenStream::new();
    let mut cv_arg_param = TokenStream::new();
    for arg in &mut func.sig.inputs {
        let mut argument_attrs = ArgumentAttrs::default();

        let pat_ty = match arg {
            syn::FnArg::Receiver(_) => bail!(arg => "cannot export self-taking methods as xsubs"),
            syn::FnArg::Typed(ref mut pt) => {
                pt.attrs
                    .retain(|attr| !argument_attrs.handle_path(&attr.path));
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

        if let Some(cv_span) = argument_attrs.cv {
            if !cv_arg_param.is_empty() {
                bail!(cv_span, "only 1 'cv' parameter allowed");
            }
            cv_arg_param = quote! { #arg_name: #arg_type };
            if passed_arguments.is_empty() {
                passed_arguments.extend(quote! { #arg_name });
            } else {
                passed_arguments.extend(quote! {, #arg_name });
            }
            continue;
        }

        let extracted_name = Ident::new(&format!("extracted_arg_{}", arg_name), arg_name.span());
        let deserialized_name =
            Ident::new(&format!("deserialized_arg_{}", arg_name), arg_name.span());

        let missing_message = syn::LitStr::new(
            &format!("missing required parameter: '{}'\n", arg_name),
            arg_name.span(),
        );

        let none_handling = if is_option_type(arg_type).is_some() {
            trailing_options += 1;
            quote! { ::perlmod::Value::new_undef(), }
        } else {
            // only cound the trailing options;
            trailing_options = 0;
            quote! {
                {
                    return Err(::perlmod::Value::new_string(#missing_message)
                        .into_mortal()
                        .into_raw());
                }
            }
        };

        extract_arguments.extend(quote! {
            let #extracted_name: ::perlmod::Value = match args.next() {
                Some(arg) => ::perlmod::Value::from(arg),
                None => #none_handling
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
            func.sig.inputs.len() - (!cv_arg_param.is_empty()) as usize
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
        !cv_arg_param.is_empty(),
    )?;

    let tokens = quote! {
        #func

        #wrapper_func

        #[inline(never)]
        #[allow(non_snake_case)]
        fn #impl_xs_name(#cv_arg_param) -> Result<#return_type, *mut ::perlmod::ffi::SV> {
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
        prototype: attr
            .prototype
            .or_else(|| Some(gen_prototype(func.sig.inputs.len(), trailing_options))),
    })
}

fn gen_prototype(arg_count: usize, trailing_options: usize) -> String {
    let arg_count = arg_count - trailing_options;

    let mut proto = String::with_capacity(arg_count + trailing_options + 1);

    for _ in 0..arg_count {
        proto.push('$');
    }
    if trailing_options > 0 {
        proto.push(';');
        for _ in 0..trailing_options {
            proto.push('$');
        }
    }
    proto
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
    export_public: Option<&syn::Visibility>,
    cv_arg: bool,
) -> Result<ReturnHandling, Error> {
    let return_type;
    let mut handle_return;
    let wrapper_func;

    let vis = match export_public {
        Some(vis) => quote! { #[no_mangle] #vis },
        None => quote! { #[allow(non_snake_case)] },
    };

    let (cv_arg_name, cv_arg_passed) = if cv_arg {
        (
            quote! { cv },
            quote! { ::perlmod::Value::from_raw_ref(cv as *mut ::perlmod::ffi::SV) },
        )
    } else {
        (quote! { _cv }, TokenStream::new())
    };

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

            wrapper_func = quote! {
                #[doc(hidden)]
                #vis extern "C" fn #xs_name(#pthx #cv_arg_name: *mut ::perlmod::ffi::CV) {
                    unsafe {
                        match #impl_xs_name(#cv_arg_passed) {
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
                #[doc(hidden)]
                #vis extern "C" fn #xs_name(#pthx #cv_arg_name: *mut ::perlmod::ffi::CV) {
                    unsafe {
                        match #impl_xs_name(#cv_arg_passed) {
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
                #[doc(hidden)]
                #vis extern "C" fn #xs_name(#pthx #cv_arg_name: *mut ::perlmod::ffi::CV) {
                    unsafe {
                        match #impl_xs_name(#cv_arg_passed) {
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
            1 => segs[0].ident == "Result",
            3 => segs[0].ident == "std" && segs[1].ident == "result" && segs[2].ident == "Result",
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

/// Note that we cannot handle renamed imports at all here...
pub fn is_option_type(ty: &syn::Type) -> Option<&syn::Type> {
    if let syn::Type::Path(p) = ty {
        if p.qself.is_some() {
            return None;
        }
        let segs = &p.path.segments;
        let is_option = match segs.len() {
            1 => segs[0].ident == "Option",
            3 => segs[0].ident == "std" && segs[1].ident == "option" && segs[2].ident == "Option",
            _ => false,
        };
        if !is_option {
            return None;
        }

        if let syn::PathArguments::AngleBracketed(generic) = &segs.last().unwrap().arguments {
            if generic.args.len() != 1 {
                return None;
            }

            if let syn::GenericArgument::Type(ty) = generic.args.first().unwrap() {
                return Some(ty);
            }
        }
    }
    None
}
