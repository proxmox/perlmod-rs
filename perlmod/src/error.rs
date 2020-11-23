use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
#[error("wrong type")]
pub struct CastError;

#[derive(ThisError, Clone, Debug)]
#[error("error: {0}")]
pub struct Error(pub(crate) String);

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
