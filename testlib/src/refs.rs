use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct TestStruct {
    text: String,

    reference: perlmod::Value,
}

#[perlmod::package(name = "TestLib::Refs", lib = "testlib")]
mod export {
    use anyhow::Error;
    use perlmod::Value;

    #[export]
    fn get_ref_from_test_struct(data: super::TestStruct) -> Result<(String, Value), Error> {
        Ok((format!("test text: {:?}", data.text), data.reference))
    }

    #[export(raw_return)]
    fn get_substr(#[raw] value: Value) -> Result<Value, Error> {
        Ok(value.substr(3..6)?)
    }
}
