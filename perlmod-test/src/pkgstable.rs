use failure::{bail, Error};

#[perlmod::export]
fn foo(a: u32, b: u32) -> Result<u32, Error> {
    if a == 42 {
        bail!("dying on magic number");
    }

    Ok(a + b)
}

#[perlmod::export(name = "xs_a")]
fn func_b(a: u32) -> Result<u32, Error> {
    Ok(a * 2)
}

perlmod::make_package! {
    package "RSPM::Foo";

    //lib "perlmod_test";

    subs {
        foo,
        xs_a as b, // func_b's exported xsub was renamed to xs_a, and in perl it's called b
    }
}
