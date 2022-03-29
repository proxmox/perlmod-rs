#[perlmod::package(name = "RSPM::Magic", lib = "perlmod_test", write = true)]
mod export {
    use perlmod::{Error, Value};

    perlmod::declare_magic!(Box<Magic> : &Magic as "RSPM::Magic");

    struct Magic {
        content: String,
    }

    impl Drop for Magic {
        fn drop(&mut self) {
            println!("Dropping blessed magic with content {:?}", self.content);
        }
    }

    #[export(raw_return)]
    fn new(#[raw] class: Value, content: String) -> Result<Value, Error> {
        Ok(perlmod::instantiate_magic!(&class, MAGIC => Box::new(Magic { content })))
    }

    #[export]
    fn call(#[try_from_ref] this: &Magic) -> Result<(), Error> {
        println!("Calling magic with content {:?}", this.content);
        Ok(())
    }
}
