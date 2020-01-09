#[derive(Debug)]
pub struct CastError;

impl std::fmt::Display for CastError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "wrong type")
    }
}

impl std::error::Error for CastError {}

#[derive(Clone, Debug)]
pub struct Error(String);

impl Error {
    #[inline]
    pub fn new(s: &str) -> Self {
        Self(s.to_string())
    }

    #[inline]
    pub fn fail<T>(s: &str) -> Result<T, Self> {
        Err(Self(s.to_string()))
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "error: {}", self.0)
    }
}

impl std::error::Error for Error {}

impl serde::de::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Self(msg.to_string())
    }
}

impl serde::ser::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Self(msg.to_string())
    }
}
