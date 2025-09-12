#[perlmod::package(name = "TestLib::Magic", lib = "testlib")]
mod export {
    use std::sync::atomic::{AtomicBool, Ordering};

    use perlmod::{Error, Value};

    static DROPPED: AtomicBool = AtomicBool::new(false);

    perlmod::declare_magic!(Box<Magic> : &Magic as "TestLib::Magic");

    struct Magic {
        content: String,
    }

    impl Drop for Magic {
        fn drop(&mut self) {
            DROPPED.store(true, Ordering::Relaxed);
        }
    }

    #[export(raw_return)]
    fn new(#[raw] class: Value, content: String) -> Result<Value, Error> {
        Ok(perlmod::instantiate_magic!(&class, MAGIC => Box::new(Magic { content })))
    }

    #[export]
    fn call(#[try_from_ref] this: &Magic) -> String {
        format!("magic box content {:?}", this.content)
    }

    #[export]
    fn is_dropped() -> bool {
        DROPPED.load(Ordering::Relaxed)
    }
}
