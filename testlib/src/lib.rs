//! Module for testing perlmod.

pub mod bless_box;
pub mod bless_magic;
pub mod digest;
pub mod errors;
pub mod refs;
pub mod ret;

#[perlmod::package(name = "TestLib::Lib", lib = "testlib")]
mod main_lib {}

#[perlmod::package(name = "TestLib::Hello", lib = "testlib", boot = "loaded")]
mod export {
    use std::sync::atomic::{AtomicBool, Ordering};

    use anyhow::{Error, bail};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Deserialize, Serialize)]
    pub struct NewString(String);

    #[derive(Deserialize, Serialize)]
    pub struct StructOptString {
        value: Option<bool>,
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
        In1(String),
        Out1(String),
    }

    static LOADED: AtomicBool = AtomicBool::new(false);

    fn loaded() {
        LOADED.store(true, Ordering::Relaxed);
    }

    #[export]
    pub fn hello(name: &str) -> Result<String, Error> {
        if !LOADED.load(Ordering::Relaxed) {
            bail!("bootstrap did not run the 'loaded' function");
        }

        Ok(format!("Hello, '{name}'"))
    }

    #[export]
    fn multi_return() -> Result<(u32, u32), std::convert::Infallible> {
        Ok((17, 32))
    }

    #[export]
    fn opt_string(arg: Option<String>) -> String {
        match arg {
            Some(arg) => format!("Called with {arg:?}."),
            None => "Called with None.".to_string(),
        }
    }

    #[export]
    fn opt_str(arg: Option<&str>) -> String {
        match arg {
            Some(arg) => format!("Called with {arg:?}."),
            None => "Called with None.".to_string(),
        }
    }

    #[export]
    fn new_string_param(value: NewString) -> String {
        format!("string contained {:?}", value.0)
    }

    #[export]
    pub fn opt_bool_to_string(value: Option<bool>) -> String {
        format!("{value:?}")
    }

    #[export]
    pub fn struct_opt_to_string(s: StructOptString) -> String {
        format!("{:?}", s.value)
    }

    #[export]
    fn map_an_enum(data: AnEnum) -> Result<AnEnum, Error> {
        Ok(match data {
            AnEnum::Something => AnEnum::ResultA,
            AnEnum::Another => AnEnum::ResultB,
            _ => bail!("invalid"),
        })
    }

    #[export]
    fn map_tagged_enum(data: TaggedEnum) -> Result<TaggedEnum, Error> {
        Ok(match data {
            TaggedEnum::In1(word) => TaggedEnum::Out1(format!("{word}.")),
            TaggedEnum::Out1(_) => bail!("out1"),
        })
    }

    #[export]
    fn trailing_optional(first: u32, second: Option<u32>) -> String {
        format!("{first:?}, {second:?}")
    }
}
