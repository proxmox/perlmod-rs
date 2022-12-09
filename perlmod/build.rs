extern crate cc;

use std::path::Path;
use std::process::Command;
use std::{env, fs, io};

fn main() {
    let out_dir = env::var("OUT_DIR").expect("expected OUT_DIR to be set by cargo");

    let include_dir = format!("{out_dir}/include");
    let ppport_h_file = format!("{include_dir}/ppport.h");
    // quoted, without exterial double qoutes
    let ppport_h_file_string_inner = ppport_h_file.replace('"', "\\\"");

    if let Err(err) = fs::create_dir(Path::new(&include_dir)) {
        if err.kind() != io::ErrorKind::AlreadyExists {
            panic!("failed to make include dir in OUT_DIR");
        }
    }

    // perl -MDevel::PPPort -e 'Devel::PPPort::WriteFile("include/ppport.h");'
    Command::new("perl")
        .arg("-MDevel::PPPort")
        .arg("-e")
        .arg(&format!(
            r#"Devel::PPPort::WriteFile("{ppport_h_file_string_inner}");"#
        ))
        .output()
        .expect("failed to create ppport.h file using perl's Devel::PPPort");

    // get include path:
    //     perl -MConfig -e 'print $Config{archlib}'
    let perl_archlib = Command::new("perl")
        .arg("-MConfig")
        .arg("-e")
        .arg("print $Config{archlib}")
        .output()
        .expect("failed to get perl arch include directory");
    // technically not a true Path, but we expect a system path which should be utf8-compatible
    let archlib_include_path = format!(
        "{}/CORE",
        std::str::from_utf8(&perl_archlib.stdout).expect("expected perl include path to be utf8")
    );

    // get perl cflags:
    //     perl -MConfig -e 'print $Config{ccflags}'
    let perl_ccflags = Command::new("perl")
        .arg("-MConfig")
        .arg("-e")
        .arg("print $Config{ccflags}")
        .output()
        .expect("failed to get perl cflags");
    // technically garbage as it may contain paths, but since this should only contain system
    // paths and otherwise also might contain quotes and what not let's just not really care:
    let ccflags = std::str::from_utf8(&perl_ccflags.stdout).expect("expected cflags to be utf8");

    let mut cc = cc::Build::new();

    cc.pic(true)
        .shared_flag(false)
        .opt_level(3)
        .include(include_dir)
        .include(archlib_include_path);

    for flag in ccflags.split_ascii_whitespace() {
        cc.flag(flag);
    }

    // now build the static library:
    cc.file("src/glue.c").compile("libglue.a");

    // get perl's MULTIPLICITY flag:
    //     perl -MConfig -e 'print $Config{usemultiplicity}'
    let perl_multiplicity = Command::new("perl")
        .arg("-MConfig")
        .arg("-e")
        .arg("print $Config{usemultiplicity}")
        .output()
        .expect("failed to get perl usemultiplicity flag");

    // pass the multiplicity cfg flag:
    if perl_multiplicity.stdout == b"define" {
        println!("cargo:rustc-cfg=perlmod=\"multiplicity\"");
    }

    // the debian package should include src/glue.c
    println!(
        "dh-cargo:deb-built-using=glue=1={}",
        env::var("CARGO_MANIFEST_DIR").unwrap()
    );
}
