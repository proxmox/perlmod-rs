use std::collections::HashMap;
use std::convert::TryFrom;
use std::env;
use std::fs::File;
use std::path::PathBuf;

use anyhow::Error;

use proc_macro2::{Ident, Span};

use syn::AttributeArgs;

use toml::Value;

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

        my $auto_path = ($pkg =~ s!::!/!gr);
        my ($mod_name) = {{LIB_NAME}};

        my @dirs = (map "-L$_/auto/$auto_path", @INC);
        my (@mod_files) = DynaLoader::dl_findfile(@dirs, '-L./target/debug', $mod_name);
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
        let mut source = format!("package {};\n{}", self.attrs.package_name, MODULE_HEAD);

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
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, source.as_bytes())?;

        Ok(())
    }

    pub fn mangle_package_name(&self) -> String {
        self.attrs.mangle_package_name()
    }
}

fn read_cargo_toml(why: Span) -> Result<HashMap<String, Value>, syn::Error> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .map_err(|err| format_err!(why, "failed to get CARGO_MANIFEST_DIR variable: {}", err))?;
    let cargo_toml_path = PathBuf::from(manifest_dir).join("Cargo.toml");

    use std::io::Read;
    let mut content = String::new();
    File::open(cargo_toml_path)
        .map_err(|err| format_err!(why, "failed to open Cargo.toml: {}", err))?
        .read_to_string(&mut content)
        .map_err(|err| format_err!(why, "failed to read Cargo.toml: {}", err))?;

    toml::from_str(&content).map_err(|err| format_err!(why, "failed to parse Cargo.toml: {}", err))
}

static mut LIB_NAME: Option<String> = None;
pub fn get_default_lib_name(why: Span) -> Result<&'static str, syn::Error> {
    unsafe {
        if let Some(name) = &LIB_NAME {
            return Ok(&name);
        }
    }

    let cargo = read_cargo_toml(why)?;

    let package = cargo
        .get("package")
        .ok_or_else(|| format_err!(
            why,
            "did not find a [package] section in Cargo.toml, try to specify the library name manually",
        ))?;

    let name = package.get("name").ok_or_else(|| {
        format_err!(
        why,
        "failed to find the package name in Cargo.toml, try to specify the library name manually",
    )
    })?;

    let name = name.as_str().ok_or_else(|| {
        format_err!(
            why,
            "package name in Cargo.toml is not a string, try to specify the library name manually",
        )
    })?;

    unsafe {
        LIB_NAME = Some(name.replace('-', "_"));
        return Ok(&LIB_NAME.as_ref().unwrap());
    }
}
