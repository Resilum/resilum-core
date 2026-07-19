use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

/// Errors surfaced by the node API.
#[derive(Debug)]
pub enum Error {
    AlreadyRunning,
    NotRunning,
    Config(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::AlreadyRunning => write!(f, "node already running"),
            Error::NotRunning => write!(f, "node not running"),
            Error::Config(m) => write!(f, "config error: {m}"),
        }
    }
}

impl std::error::Error for Error {}
