use serde::{Deserialize, Serialize};

use perlmod::Value;

#[derive(Debug, Deserialize, Serialize)]
pub struct Blubber(String);

#[derive(Debug, Deserialize, Serialize)]
pub struct RawRefs {
    copied: String,

    reference: Value,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
enum AnEnum {
    Something,
    Another,
    ResultA,
    ResultB,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
enum TaggedEnum {
    Something(String),
    Another(i32, i32),
}

#[perlmod::package(
    name = "RSPM::Foo142",
    lib = "perlmod_test",
    write = true,
    boot = "loaded"
)]
mod export {
    use anyhow::{bail, Error};

    use perlmod::Value;

    use super::AnEnum;
    use super::TaggedEnum;

    fn loaded() {
        println!("<loaded>");
    }

    #[export]
    fn foo142(a: u32, b: u32) -> Result<u32, Error> {
        if a == 42 {
            bail!("dying on magic number");
        }

        Ok(a + b)
    }

    #[export]
    fn test(t: Option<String>) -> Result<(), Error> {
        println!("test called with {t:?}");
        Ok(())
    }

    #[export]
    fn teststr(t: Option<&str>) -> Result<(), Error> {
        println!("teststr called with {t:?}");
        Ok(())
    }

    #[export]
    fn test_serde(value: super::Blubber) -> Result<String, Error> {
        println!("got {value:?}");
        Ok(value.0)
    }

    #[export]
    fn test_refs(data: super::RawRefs) -> Result<Value, Error> {
        println!("test_refs: copied text: {:?}", data.copied);
        Ok(data.reference)
    }

    #[export]
    fn test_enums(data: AnEnum) -> Result<AnEnum, Error> {
        Ok(match data {
            AnEnum::Something => AnEnum::ResultA,
            AnEnum::Another => AnEnum::ResultB,
            _ => bail!("invalid"),
        })
    }

    #[export]
    fn test_enums2(data: TaggedEnum) -> TaggedEnum {
        match data {
            TaggedEnum::Something(word) => TaggedEnum::Something(format!("{word}.")),
            TaggedEnum::Another(n, m) => TaggedEnum::Another(n + 1, m * 2),
        }
    }

    #[export]
    fn test_trailing_optional(first: u32, second: Option<u32>) {
        println!("{first:?}, {second:?}");
    }

    #[export(xs_name = "testit_xsub")]
    fn testit(#[cv] cv: Value, arg: &str) {
        let _ = (cv, arg);
    }

    #[export(raw_return)]
    fn test_substr_return(#[raw] value: Value) -> Result<Value, Error> {
        Ok(value.substr(3..6)?)
    }

    #[derive(serde::Serialize)]
    struct MyError {
        a: String,
        b: String,
    }

    #[export(serialize_error, errno)]
    fn test_deserialized_error(fail: bool) -> Result<&'static str, MyError> {
        if fail {
            Err(MyError {
                a: "first".to_string(),
                b: "second".to_string(),
            })
        } else {
            ::perlmod::error::set_errno(77);
            Ok("worked")
        }
    }

    #[export]
    fn use_safe_putenv(on: bool) {
        perlmod::ffi::use_safe_putenv(on);
    }

    #[export]
    fn set_env(name: &str, value: &str) {
        unsafe { std::env::set_var(name, value) };
    }

    #[export]
    fn unset_env(name: &str) {
        unsafe { std::env::remove_var(name) };
    }
}

#[perlmod::package(name = "RSPM::EnvVarLibrary", lib = "x-${CARGO_PKG_NAME}-y")]
mod expanded_export {
    use anyhow::Error;

    #[export]
    fn test_lib_env_vars(value: &str) -> Result<(), Error> {
        println!("foo: {value:?}");
        Ok(())
    }
}
