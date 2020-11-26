#[perlmod::package(name = "RSPM::Bless", lib = "perlmod_test")]
mod export {
    use anyhow::Error;

    use perlmod::Value;

    const CLASSNAME: &str = "RSPM::Bless";

    struct Bless {
        content: String,
    }

    #[export(raw_return)]
    fn new(#[raw] class: Value, content: String) -> Result<Value, Error> {
        let mut ptr = Box::new(Bless { content });

        let value = Value::new_pointer::<Bless>(&mut *ptr);
        let value = Value::new_ref(&value);
        let this = value.bless_sv(&class)?;
        let _perl = Box::leak(ptr);

        Ok(this)
    }

    #[export]
    fn something(#[raw] this: Value) -> Result<(), Error> {
        let this = unsafe { this.from_blessed_box::<Bless>(CLASSNAME)? };
        println!("Called something on Bless {{ {:?} }}!", this.content);
        Ok(())
    }

    perlmod::destructor! { Bless : CLASSNAME }
}
