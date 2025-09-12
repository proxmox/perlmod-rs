use std::path::Path;
use std::process::Command;

use anyhow::{Context as _, Error, bail};

#[test]
fn main() -> Result<(), Error> {
    let manifest_dir =
        std::env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR variable missing");
    let manifest_dir = Path::new(&manifest_dir);

    let perlmod_gen_dir =
        std::env::var_os("PERLMOD_GEN_DIR").expect("PERLMOD_GEN_DIR variable missing");
    let perlmod_lib_dir =
        std::env::var_os("PERLMOD_LIB_DIR").expect("PERLMOD_LIB_DIR variable missing");

    let mut ld_libray_path = std::env::var_os("LD_LIBRARY_PATH").unwrap_or_default();
    if !ld_libray_path.is_empty() {
        ld_libray_path.push(":");
    }
    ld_libray_path.push(perlmod_lib_dir);
    unsafe {
        std::env::set_var("LD_LIBRARY_PATH", ld_libray_path);
    }

    let mut test_list = std::fs::read_dir(manifest_dir)
        .context("failed to list contents of {manifest_dir:?}")?
        .try_fold(Vec::new(), |mut acc, item| {
            let item = item.context("error listing directory")?;
            if let Ok(name) = item.file_name().into_string() {
                if !name.starts_with('.') & name.ends_with(".t") {
                    acc.push(name);
                }
            }
            Ok::<_, Error>(acc)
        })?;

    test_list.sort();

    let mut failed = false;
    for test in test_list {
        let path = manifest_dir.join(&test);
        if !Command::new("/usr/bin/perl")
            .arg("-I")
            .arg(&perlmod_gen_dir)
            .arg(path)
            .status()
            .with_context(|| format!("perl invocation failed for test {test:?}"))?
            .success()
        {
            failed = true;
            eprintln!("error in test {test:?}");
        }
    }

    if failed {
        bail!("there were errors (lib-dir = {perlmod_gen_dir:?})");
    }

    Ok(())
}
