use std::convert::TryFrom;
use std::env;

use proc_macro2::{Ident, Span};

use syn::Error;
use syn::AttributeArgs;

use crate::attribs::ModuleAttrs;

const MODULE_HEAD: &str = r#"
use strict;
use warnings;
use DynaLoader ();

my $LIB;

BEGIN {
    my sub newXS {
        my ($perl_func_name, $full_symbol_name, $filename) = @_;

        my $sym  = DynaLoader::dl_find_symbol($LIB, $full_symbol_name);
        die "failed to locate '$full_symbol_name'\n" if !defined $sym;
        DynaLoader::dl_install_xsub($perl_func_name, $sym, $filename);
    }

    my sub __load_shared_lib {
        return if $LIB;

        my ($pkg) = @_;

        my ($mod_name) = {{LIB_NAME}};

        my @dirs = (map "-L$_/auto", @INC);
        my (@mod_files) = DynaLoader::dl_findfile(@dirs"#;

#[cfg(debug_assertions)]
const MODULE_HEAD_DEBUG: &str = r#", '-L./target/debug'"#;

#[cfg(not(debug_assertions))]
const MODULE_HEAD_DEBUG: &str = "";

const MODULE_HEAD_2: &str = r#", $mod_name);
        die "failed to locate shared library for '$pkg' (lib${mod_name}.so)\n" if !@mod_files;

        $LIB = DynaLoader::dl_load_file($mod_files[0])
            or die "failed to load library '$mod_files[0]'\n";
    }

    __load_shared_lib(__PACKAGE__);
"#;

const MODULE_TAIL: &str = "}\n";

struct Export {
    rust_name: Ident,
    perl_name: Option<Ident>,
    xs_name: Ident,
    file_name: String,
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

    pub fn export_named(
        &mut self,
        rust_name: Ident,
        perl_name: Option<Ident>,
        xs_name: Ident,
        file_name: String,
    ) {
        self.exported.push(Export {
            rust_name,
            perl_name,
            xs_name,
            file_name,
        });
    }

    pub fn write(&self) -> Result<(), Error> {
        let mut source = format!(
            "package {};\n{}{}{}",
            self.attrs.package_name, MODULE_HEAD, MODULE_HEAD_DEBUG, MODULE_HEAD_2
        );

        for export in &self.exported {
            source = format!(
                "{}    newXS('{}', '{}', \"{}\");\n",
                source,
                export.perl_name.as_ref().unwrap_or(&export.rust_name),
                export.xs_name,
                export.file_name.replace('"', "\\\""),
            );
        }

        source.push_str(MODULE_TAIL);

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
