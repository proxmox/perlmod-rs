use std::convert::TryFrom;

use failure::Error;

use proc_macro2::Ident;

use syn::AttributeArgs;

use crate::attribs::ModuleAttrs;

const LIB_NAME_DEFAULT: &str = r#"($pkg =~ /(?:^|::)([^:]+)$/)"#;

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
            source = source.replace("{{LIB_NAME}}", LIB_NAME_DEFAULT);
        }

        let path = std::path::Path::new(&self.attrs.file_name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, source.as_bytes())?;

        Ok(())
    }
}
