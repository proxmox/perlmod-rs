#[perlmod::package(name = "RSPM::Option", lib = "perlmod_test", write = true)]
mod export {
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize, Serialize)]
    pub struct WithOption {
        tristate: Option<bool>,
    }

    #[export]
    pub fn to_string(tristate: Option<bool>) -> String {
        format!("{:?}", tristate)
    }

    #[export]
    pub fn struct_to_string(with_option: WithOption) -> String {
        format!("{:?}", with_option.tristate)
    }
}
