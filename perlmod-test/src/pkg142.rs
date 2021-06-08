#[perlmod::package(name = "RSPM::Foo142", lib = "perlmod_test")]
mod export {
    use anyhow::{bail, Error};

    #[export]
    fn foo142(a: u32, b: u32) -> Result<u32, Error> {
        if a == 42 {
            bail!("dying on magic number");
        }

        Ok(a + b)
    }

    #[export]
    fn test(t: Option<String>) -> Result<(), Error> {
        println!("test called with {:?}", t);
        Ok(())
    }

    #[export]
    fn teststr(t: Option<&str>) -> Result<(), Error> {
        println!("teststr called with {:?}", t);
        Ok(())
    }
}
