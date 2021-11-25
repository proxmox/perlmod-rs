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

#[perlmod::package(name = "RSPM::Foo142", lib = "perlmod_test")]
mod export {
    use anyhow::{bail, Error};

    use perlmod::Value;

    use super::AnEnum;

    #[export]
    fn foo142(a: u32, b: u32) -> Result<u32, Error> {
        if a == 42 {
            bail!("dying on magic number");
        }

        Ok(a + b)
    }

    #[export]
    fn test(t: Option<String>) -> Result<(), Error> {
        println!("test called with {:?}", t);
        Ok(())
    }

    #[export]
    fn teststr(t: Option<&str>) -> Result<(), Error> {
        println!("teststr called with {:?}", t);
        Ok(())
    }

    #[export]
    fn test_serde(value: super::Blubber) -> Result<String, Error> {
        println!("got {:?}", value);
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
    fn test_trailing_optional(first: u32, second: Option<u32>) {
        println!("{:?}, {:?}", first, second);
    }
}

#[perlmod::package(name = "RSPM::EnvVarLibrary", lib = "x-${CARGO_PKG_NAME}-y")]
mod expanded_export {
    use anyhow::Error;

    #[export]
    fn test_lib_env_vars(value: &str) -> Result<(), Error> {
        println!("foo: {:?}", value);
        Ok(())
    }
}
