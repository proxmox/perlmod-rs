use std::convert::TryFrom;
use std::env;

use proc_macro2::{Ident, Span, TokenStream};

use quote::quote;
use syn::AttributeArgs;
use syn::Error;

use crate::attribs::ModuleAttrs;

const MODULE_HEAD: &str = r#"
require DynaLoader;

sub bootstrap {
    my ($pkg) = @_;
    my ($mod_name) = {{LIB_NAME}};
    my $bootstrap_name = 'xs_bootstrap_' . ($pkg =~ s/::/__/gr);

    my @dirs = (map "-L$_/auto", @INC);
    my $mod_file = DynaLoader::dl_findfile("#;

#[cfg(debug_assertions)]
const MODULE_HEAD_DEBUG: &str = r#"'-L./target/debug', "#;

#[cfg(not(debug_assertions))]
const MODULE_HEAD_DEBUG: &str = "";

const MODULE_HEAD_2: &str = r#"@dirs, $mod_name);
    die "failed to locate shared library for '$pkg' (lib${mod_name}.so)\n" if !$mod_file;

    my $lib = DynaLoader::dl_load_file($mod_file)
        or die "failed to load library '$mod_file'\n";

    my $sym  = DynaLoader::dl_find_symbol($lib, $bootstrap_name);
    die "failed to locate '$bootstrap_name'\n" if !defined $sym;
    my $boot = DynaLoader::dl_install_xsub($bootstrap_name, $sym, "src/FIXME.rs");
    $boot->();
}

__PACKAGE__->bootstrap;

1;
"#;

struct Export {
    rust_name: Ident,
    perl_name: Option<Ident>,
    xs_name: Ident,
}

pub struct Package {
    attrs: ModuleAttrs,
    exported: Vec<Export>,
}

impl Package {
    pub fn with_attrs(attr: AttributeArgs) -> Result<Self, Error> {
        Ok(Self {
            attrs: ModuleAttrs::try_from(attr)?,
            exported: Vec::new(),
        })
    }

    pub fn export_named(&mut self, rust_name: Ident, perl_name: Option<Ident>, xs_name: Ident) {
        self.exported.push(Export {
            rust_name,
            perl_name,
            xs_name,
        });
    }

    pub fn bootstrap_function(&self) -> TokenStream {
        let mut newxs = TokenStream::new();
        for export in &self.exported {
            let perl_name = export.perl_name.as_ref().unwrap_or(&export.rust_name);
            let sub_name = format!("{}::{}\0", self.attrs.package_name, perl_name);
            let sub_lit = syn::LitByteStr::new(sub_name.as_bytes(), perl_name.span());

            let xs_name = &export.xs_name;

            newxs.extend(quote! {
                RSPL_newXS_flags(
                    #sub_lit.as_ptr() as *const i8,
                    #xs_name as _,
                    concat!(::std::file!(), "\0").as_bytes().as_ptr() as *const i8,
                    ::std::ptr::null(), // TODO: perl-style prototype string goes here
                    0,
                );
            });
        }

        let bootstrap_name =
            format!("xs_bootstrap_{}", self.attrs.package_name).replace("::", "__");
        let bootstrap_ident = Ident::new(&bootstrap_name, Span::call_site());

        quote! {
            #[no_mangle]
            pub extern "C" fn #bootstrap_ident(
                _cv: &::perlmod::ffi::CV,
            ) {
                unsafe {
                    use ::perlmod::ffi::RSPL_newXS_flags;

                    let argmark = ::perlmod::ffi::pop_arg_mark();
                    argmark.set_stack();

                    #newxs
                }
            }
        }
    }

    pub fn write(&self) -> Result<(), Error> {
        let mut source = format!(
            "package {};\n{}{}{}",
            self.attrs.package_name, MODULE_HEAD, MODULE_HEAD_DEBUG, MODULE_HEAD_2
        );

        if let Some(lib) = &self.attrs.lib_name {
            source = source.replace("{{LIB_NAME}}", &format!("('{}')", lib));
        } else {
            let lib_name = get_default_lib_name(Span::call_site())?;
            source = source.replace("{{LIB_NAME}}", &format!("('{}')", lib_name));
        }

        let file_name = self
            .attrs
            .file_name
            .clone()
            .unwrap_or_else(|| format!("{}.pm", self.attrs.package_name.replace("::", "/")));

        let path = std::path::Path::new(&file_name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(io_err)?;
        }
        std::fs::write(path, source.as_bytes()).map_err(io_err)?;

        Ok(())
    }

    pub fn mangle_package_name(&self) -> String {
        self.attrs.mangle_package_name()
    }
}

fn io_err<E: ToString>(err: E) -> Error {
    Error::new(Span::call_site(), err.to_string())
}

pub fn get_default_lib_name(why: Span) -> Result<String, Error> {
    env::var("CARGO_PKG_NAME")
        .map(|s| s.replace('-', "_"))
        .map_err(|err| {
            format_err!(
                why,
                "failed to get CARGO_PKG_NAME environment variable: {}",
                err
            )
        })
}
