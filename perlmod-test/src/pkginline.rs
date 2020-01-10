#![perlmod::package(name = "RSPM::Foo", lib = "perlmod_test")]

use failure::{bail, Error};

#[export]
fn foo(a: u32, b: u32) -> Result<u32, Error> {
    if a == 42 {
        bail!("dying on magic number");
    }

    Ok(a + b)
}
