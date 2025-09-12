#[perlmod::package(name = "TestLib::BlessBox")]
mod export {
    use std::cell::RefCell;
    use std::panic::AssertUnwindSafe as Aus;

    use anyhow::Error;
    use serde::Serialize;

    use perlmod::Value;

    const CLASSNAME: &str = "TestLib::BlessBox";

    struct BlessBox {
        inner: Aus<RefCell<Inner>>,
        guard: Value,
    }

    #[derive(Clone, Serialize)]
    struct Inner {
        content: String,
    }

    impl<'a> TryFrom<&'a Value> for &'a BlessBox {
        type Error = Error;

        fn try_from(value: &'a Value) -> Result<&'a BlessBox, Error> {
            Ok(unsafe { value.from_blessed_box(CLASSNAME)? })
        }
    }

    #[export(raw_return)]
    fn new(
        #[raw] class: Value,
        content: String,
        #[raw] guard: Value,
    ) -> Result<Value, perlmod::Error> {
        Value::bless_box(
            class,
            Box::new(BlessBox {
                inner: Aus(RefCell::new(Inner { content })),
                guard,
            }),
        )
    }

    // The `#[raw]` attribute is an optimization, but not strictly required.
    #[export]
    fn raw_method(#[raw] this: Value) -> Result<Inner, Error> {
        let this = unsafe { this.from_blessed_box::<BlessBox>(CLASSNAME)? };

        Ok(this.inner.borrow_mut().clone())
    }

    #[export]
    fn method(this: Value) -> Result<Inner, Error> {
        let this = unsafe { this.from_blessed_box::<BlessBox>(CLASSNAME)? };

        Ok(this.inner.borrow_mut().clone())
    }

    #[export]
    fn update(#[try_from_ref] this: &BlessBox, param: u32) -> Result<(), Error> {
        this.inner.borrow_mut().content = format!("updated to {param}");
        Ok(())
    }

    #[export(name = "DESTROY")]
    fn destroy(#[raw] this: Value) {
        {
            let this = unsafe {
                this.from_blessed_box::<BlessBox>(CLASSNAME)
                    .expect("blessed value conversion failed")
            };
            this.guard
                .dereference()
                .expect("guard was not a reference")
                .as_hash()
                .expect("guard was not a hash")
                .insert("destroyed", Value::new_uint(44));
        }
        perlmod::destructor!(this, BlessBox: CLASSNAME);
    }
}
