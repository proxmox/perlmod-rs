#[perlmod::package(name = "RSPM::Bless", lib = "perlmod_test", write = true)]
mod export {
    use anyhow::Error;

    use perlmod::Value;

    const CLASSNAME: &str = "RSPM::Bless";

    struct Bless {
        content: String,
    }

    #[export(raw_return)]
    fn new(#[raw] class: Value, content: String) -> Result<Value, perlmod::Error> {
        Value::bless_box(class, Box::new(Bless { content }))
    }

    // The `#[raw]` attribute is an optimization, but not strictly required.
    #[export]
    fn something(#[raw] this: Value) -> Result<(), Error> {
        let this = unsafe { this.from_blessed_box::<Bless>(CLASSNAME)? };
        println!("Called something on Bless {{ {:?} }}!", this.content);
        Ok(())
    }

    #[export]
    fn something_nonraw(this: Value) -> Result<(), Error> {
        let this = unsafe { this.from_blessed_box::<Bless>(CLASSNAME)? };
        println!("Called something_nonraw on Bless {{ {:?} }}!", this.content);
        Ok(())
    }

    #[export]
    fn another(#[try_from_ref] this: &Bless, param: u32) -> Result<(), Error> {
        println!(
            "Called 'another({})' on Bless {{ {:?} }}!",
            param, this.content
        );
        Ok(())
    }

    #[export(name = "DESTROY")]
    fn destroy(#[raw] this: Value) {
        perlmod::destructor!(this, Bless: CLASSNAME);
    }

    #[export]
    fn multi_return(#[raw] _this: Value) -> Result<(u32, u32), std::convert::Infallible> {
        Ok((17, 32))
    }

    impl<'a> TryFrom<&'a Value> for &'a Bless {
        type Error = Error;

        fn try_from(value: &'a Value) -> Result<&'a Bless, Error> {
            Ok(unsafe { value.from_blessed_box(CLASSNAME)? })
        }
    }
}
