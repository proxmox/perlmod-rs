use std::collections::HashMap;
use std::convert::TryFrom;
use std::env;
use std::fs::File;
use std::path::PathBuf;

use anyhow::Error;

use proc_macro2::{Ident, Span};

use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::AttributeArgs;
use syn::Token;

use toml::Value;

use crate::attribs::ModuleAttrs;

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

struct Export {
    rust_name: Ident,
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

    pub fn export_named(&mut self, rust_name: Ident, xs_name: Ident, file_name: String) {
        self.exported.push(Export {
            rust_name,
            xs_name,
            file_name,
        });
    }

    pub fn export_direct(&mut self, name: Ident, file_name: String) {
        let xs_name = Ident::new(&format!("xs_{}", name), name.span());
        self.exported.push(Export {
            rust_name: name,
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
                export.rust_name,
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
}

mod kw {
    syn::custom_keyword!(package);
    syn::custom_keyword!(lib);
    syn::custom_keyword!(file);
    syn::custom_keyword!(subs);
}

impl Parse for Package {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut pkg = Package {
            attrs: ModuleAttrs {
                package_name: String::new(),
                file_name: None,
                lib_name: None,
            },
            exported: Vec::new(),
        };

        // `package "Package::Name";` comes first
        let _pkg: kw::package = input.parse()?;
        let package: syn::LitStr = input.parse()?;
        let _semicolon: Token![;] = input.parse()?;
        pkg.attrs.package_name = package.value();

        // `lib "lib_name";` optionally comes second
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::lib) {
            let _lib: kw::lib = input.parse()?;
            let lib: syn::LitStr = input.parse()?;
            pkg.attrs.lib_name = Some(lib.value());
            let _semicolon: Token![;] = input.parse()?;
        }
        drop(lookahead);

        // `file "File/Name.pm";` optionally comes third
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::file) {
            let _file: kw::file = input.parse()?;
            let file: syn::LitStr = input.parse()?;
            pkg.attrs.file_name = Some(file.value());
            let _semicolon: Token![;] = input.parse()?;
        }
        drop(lookahead);

        // `sub { ... }` must follow:
        let _sub: kw::subs = input.parse()?;
        let content;
        let _brace_token: syn::token::Brace = syn::braced!(content in input);
        let items: Punctuated<ExportItem, Token![,]> =
            content.parse_terminated(ExportItem::parse)?;

        for item in items {
            match item {
                ExportItem::Direct(name) => pkg.export_direct(name, "src/FIXME.rs".to_string()),
                ExportItem::Named(name, as_name) => {
                    pkg.export_named(as_name, name, "src/FIXME.rs".to_string());
                }
            }
        }

        Ok(pkg)
    }
}

enum ExportItem {
    Direct(Ident),
    Named(Ident, Ident),
}

impl Parse for ExportItem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        let lookahead = input.lookahead1();
        if lookahead.peek(syn::token::As) {
            let _as: syn::token::As = input.parse()?;
            Ok(ExportItem::Named(name, input.parse()?))
        } else {
            Ok(ExportItem::Direct(name))
        }
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
