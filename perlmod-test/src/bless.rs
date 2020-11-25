#[perlmod::package(name = "RSPM::Bless", lib = "perlmod_test")]
mod export {
    use anyhow::{format_err, Error};

    use perlmod::Value;

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
    fn something(#[raw] value: Value) {
        let _ = value; // ignore for now
        println!("Called something!");
    }

    #[export]
    fn DESTROY(#[raw] this: Value) {
        match this
            .dereference()
            .ok_or_else(|| format_err!("not a reference"))
            .and_then(|this| Ok(this.pv_raw()?))
        {
            Ok(ptr) => {
                let value = unsafe { Box::<Bless>::from_raw(ptr) };
                println!("Dropping value {:?}", value.content);
            }
            Err(err) => {
                println!("DESTROY called with invalid pointer: {}", err);
            }
        }
    }
}

// Example:
// use RSPM::Bless;
// my $foo = RSPM::Bless::new("Some Content");
// $foo->something(); // works
//
// output:
// Called something!
// Dropping value "Some Content"
