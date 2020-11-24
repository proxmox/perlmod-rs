#[perlmod::package(name = "RSPM::Bless", lib = "perlmod_test")]
mod export {
    use anyhow::Error;

    use perlmod::Value;

    #[export(raw_return)]
    fn new() -> Result<Value, Error> {
        let hash = Value::from(perlmod::hash::Hash::new());
        let hash = Value::new_ref(&hash);
        let hash = hash.bless("RSPM::Bless")?;
        Ok(hash)
        //Ok(this.bless("RSPM::Bless")?)
    }

    #[export]
    fn something(#[raw] value: Value) {
        println!("Called something!");
    }

    #[export]
    fn DESTROY(#[raw] this: Value) {
        println!("Value dropped!");
    }
}
