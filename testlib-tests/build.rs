use std::io::Write;
use std::path::Path;
use std::process::Command;

use anyhow::{Context as _, Error, bail};

fn main() -> Result<(), Error> {
    let manifest_dir =
        std::env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR variable missing");
    let manifest_dir = Path::new(&manifest_dir);
    let out_dir = std::env::var_os("OUT_DIR").expect("OUT_DIR variable missing");
    let out_dir = Path::new(&out_dir);

    let root_dir = manifest_dir.parent().unwrap();

    let target_dir = out_dir.join("target");
    let ld_path = target_dir.join("debug");
    let library = ld_path.join("libtestlib.so");

    build_library(root_dir, &target_dir, &library)?;

    let gen_script = root_dir.join("perlmod-bin/genpackage.pl");
    eprintln!("Running genpackage");
    if !Command::new(gen_script)
        .current_dir(out_dir)
        .args([
            "--lib-package",
            "TestLib::Lib",
            "--lib-prefix",
            "TestLib",
            "--lib-tag",
            "TestLib",
            "--lib",
            "testlib",
            "--from-notes",
        ])
        .arg(&library)
        .status()
        .context("failed to run genpackage.pl")?
        .success()
    {
        bail!("genpackage.pl failed");
    }

    set_path_var("PERLMOD_GEN_DIR", out_dir)?;
    set_path_var("PERLMOD_LIB_DIR", &ld_path)?;
    Ok(())
}

fn set_path_var(var: &'static str, path: &Path) -> Result<(), Error> {
    let mut out = std::io::stdout();
    write!(out, "cargo::rustc-env={var}=")?;
    out.write_all(path.as_os_str().as_encoded_bytes())?;
    out.write_all(b"\n")?;
    out.flush()?;
    Ok(())
}

/// FIXME: Once cargo supports "artifact dependencies" we can instead make the `cdylib` an actual
/// dependency!
fn build_library(root_dir: &Path, target_dir: &Path, library: &Path) -> Result<(), Error> {
    let cargo = std::env::var_os("CARGO").context("missing CARGO env var")?;

    if !Command::new(&cargo)
        .current_dir(root_dir)
        .arg("build")
        .arg("--lib")
        .arg("--target-dir")
        .arg(target_dir)
        .arg("-p")
        .arg("testlib")
        .status()
        .context("failed to run 'cargo build -p testlib'")?
        .success()
    {
        bail!("cargo build -p testlib failed");
    }

    if !library.exists() {
        bail!("cargo build -p testlib did not produce {library:?}");
    }

    let mut out = std::io::stdout();
    out.write_all(b"cargo::rerun-if-changed=")?;
    out.write_all(root_dir.as_os_str().as_encoded_bytes())?;
    out.write_all(b"/testlib/")?;
    out.flush()?;

    Ok(())
}
