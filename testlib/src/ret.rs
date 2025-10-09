//! Test more complex return-value code like `wantarray` support.

#[perlmod::package(name = "TestLib::Ret", lib = "testlib")]
mod export {
    use anyhow::{Error, bail};

    use perlmod::Gimme;
    use perlmod::ser::Return;

    #[export]
    fn get_tuple() -> (&'static str, &'static str) {
        ("first", "second")
    }

    #[export]
    fn maybe_many() -> Return<&'static str, Vec<&'static str>> {
        Gimme::map(|| "single value", || vec!["multiple", "values"])
    }

    #[export]
    fn try_maybe_many(
        fail: Option<&str>,
    ) -> Result<Return<&'static str, Vec<&'static str>>, Error> {
        Gimme::try_map(
            || {
                if let Some(fail) = fail {
                    bail!("failed in scalar context ({fail})")
                } else {
                    Ok("try single value")
                }
            },
            || {
                if let Some(fail) = fail {
                    bail!("failed in list context ({fail})")
                } else {
                    Ok(vec!["try", "multiple", "values"])
                }
            },
        )
    }
}
