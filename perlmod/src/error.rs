use std::fmt;

/// Error returned by `TryFrom` implementations between `Scalar`, `Array` and `Hash`.
#[derive(Debug)]
pub struct CastError;

impl std::error::Error for CastError {}

impl fmt::Display for CastError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("wrong type")
    }
}

/// Generic errors from the perlmod crate.
#[derive(Clone, Debug)]
pub struct Error(pub(crate) String);

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "error: {}", self.0)
    }
}

impl Error {
    #[inline]
    pub fn new(s: &str) -> Self {
        Self(s.to_string())
    }

    #[inline]
    pub fn new_owned(s: String) -> Self {
        Self(s)
    }

    #[inline]
    pub fn fail<T>(s: &str) -> Result<T, Self> {
        Err(Self(s.to_string()))
    }
}

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
