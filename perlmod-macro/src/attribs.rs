use proc_macro2::{Ident, Span};

use syn::punctuated::Punctuated;
use syn::{Error, Meta, Token};

pub struct ModuleAttrs {
    pub package_name: String,
    pub file_name: Option<String>,
    pub lib_name: Option<String>,
    pub write: Option<bool>,
    pub boot: Option<syn::Path>,
}

fn is_ident_check_dup<T>(path: &syn::Path, var: &Option<T>, what: &'static str) -> bool {
    if path.is_ident(what) {
        if var.is_some() {
            error!(path => "found multiple '{}' attributes", what);
        }
        true
    } else {
        false
    }
}

impl TryFrom<Punctuated<Meta, Token![,]>> for ModuleAttrs {
    type Error = Error;

    fn try_from(args: Punctuated<Meta, Token![,]>) -> Result<Self, Self::Error> {
        let mut package_name = None;
        let mut file_name = None;
        let mut lib_name = None;
        let mut write = None;
        let mut boot = None;

        for arg in args {
            let (path, value) = match arg {
                syn::Meta::NameValue(syn::MetaNameValue { path, value, .. }) => (path, value),
                _ => {
                    error!(Span::call_site(), "unexpected attribute argument");
                    continue;
                }
            };

            if is_ident_check_dup(&path, &package_name, "name") {
                let Some(litstr) = expect_lit_str(value) else {
                    continue;
                };
                package_name = Some(expand_env_vars(&litstr)?);
            } else if is_ident_check_dup(&path, &file_name, "file") {
                let Some(litstr) = expect_lit_str(value) else {
                    continue;
                };
                file_name = Some(expand_env_vars(&litstr)?);
            } else if is_ident_check_dup(&path, &lib_name, "lib") {
                let Some(litstr) = expect_lit_str(value) else {
                    continue;
                };
                lib_name = Some(expand_env_vars(&litstr)?);
            } else if is_ident_check_dup(&path, &boot, "boot") {
                let Some(litstr) = expect_lit_str(value) else {
                    continue;
                };
                boot = Some(litstr.parse::<syn::Path>()?);
            } else if is_ident_check_dup(&path, &write, "write") {
                let Some(litbool) = expect_lit_bool(value) else {
                    continue;
                };
                write = Some(litbool.value());
            } else {
                error!(path => "unknown argument");
            }
        }

        let package_name = package_name
            .ok_or_else(|| format_err!(Span::call_site(), "missing 'package' argument"))?;

        Ok(Self {
            package_name,
            file_name,
            lib_name,
            write,
            boot,
        })
    }
}

fn expand_env_vars(lit_str: &syn::LitStr) -> Result<String, Error> {
    let input = lit_str.value();
    let mut expanded = String::with_capacity(input.len());

    let mut input = input.as_str();
    loop {
        let dollar = match input.find("${") {
            Some(d) => d,
            None => {
                expanded.push_str(input);
                break;
            }
        };

        expanded.push_str(&input[..dollar]);
        input = &input[(dollar + 2)..];

        let end = input.find('}').ok_or_else(
            || format_err!(lit_str => "missing end of environment variable expansion"),
        )?;

        let var_name = &input[..end];
        input = &input[(end + 1)..];

        let var = std::env::var(var_name).map_err(|err| {
            format_err!(lit_str => "failed to expand environment variable {:?}: {}", var_name, err)
        })?;
        expanded.push_str(&var);
    }

    Ok(expanded)
}

impl ModuleAttrs {
    pub fn mangle_package_name(&self) -> String {
        let mut out = String::with_capacity(self.package_name.len());
        for ch in self.package_name.chars() {
            if ch.is_ascii_alphabetic() || ch.is_ascii_digit() {
                out.push(ch);
            } else {
                out.push('_');
            }
        }
        out
    }
}

#[derive(Default)]
pub struct FunctionAttrs {
    pub perl_name: Option<Ident>,
    pub xs_name: Option<Ident>,
    pub raw_return: bool,
    //pub cv_variable: Option<Ident>,
    pub prototype: Option<String>,
    pub serialize_error: bool,
    pub errno: bool,
}

impl TryFrom<Punctuated<Meta, Token![,]>> for FunctionAttrs {
    type Error = Error;

    fn try_from(args: Punctuated<Meta, Token![,]>) -> Result<Self, Self::Error> {
        let mut attrs = FunctionAttrs::default();

        for arg in args {
            match arg {
                syn::Meta::NameValue(syn::MetaNameValue { path, value, .. }) => {
                    let Some(litstr) = expect_lit_str(value) else {
                        continue;
                    };
                    if is_ident_check_dup(&path, &attrs.xs_name, "xs_name") {
                        attrs.xs_name = Some(Ident::new(&litstr.value(), litstr.span()));
                    } else if is_ident_check_dup(&path, &attrs.perl_name, "name") {
                        attrs.perl_name = Some(Ident::new(&litstr.value(), litstr.span()));
                    } else if is_ident_check_dup(&path, &attrs.prototype, "prototype") {
                        attrs.prototype = Some(litstr.value());
                    } else {
                        error!(path => "unknown argument");
                        continue;
                    }
                }
                syn::Meta::Path(path) => {
                    if path.is_ident("raw_return") {
                        attrs.raw_return = true;
                    } else if path.is_ident("serialize_error") {
                        attrs.serialize_error = true;
                    } else if path.is_ident("errno") {
                        attrs.errno = true;
                    } else {
                        error!(path => "unknown attribute");
                    }
                }
                _ => error!(Span::call_site(), "unexpected attribute argument"),
            }
        }

        Ok(attrs)
    }
}

fn expect_lit_str(value: syn::Expr) -> Option<syn::LitStr> {
    match value {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(lit),
            ..
        }) => Some(lit),
        _ => {
            error!(value => "value must be a literal string");
            None
        }
    }
}

fn expect_lit_bool(value: syn::Expr) -> Option<syn::LitBool> {
    match value {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Bool(lit),
            ..
        }) => Some(lit),
        _ => {
            error!(value => "value must be a literal boolean");
            None
        }
    }
}
