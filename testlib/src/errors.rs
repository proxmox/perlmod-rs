//! Testing structured errors.

#[perlmod::package(name = "TestLib::Errors", lib = "testlib")]
mod export {
    use serde::Serialize;

    #[derive(Serialize)]
    struct MyError {
        a: String,
        b: String,
    }

    #[export(serialize_error, errno)]
    fn my_error(fail: bool) -> Result<&'static str, MyError> {
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
}
