use std::fmt;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Hyper(hyper::Error),
    InvalidAddress(String),
    Other(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "io error: {e}"),
            Error::Hyper(e) => write!(f, "hyper error: {e}"),
            Error::InvalidAddress(msg) => write!(f, "invalid address: {msg}"),
            Error::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::Io(value)
    }
}

impl From<hyper::Error> for Error {
    fn from(value: hyper::Error) -> Self {
        Error::Hyper(value)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
