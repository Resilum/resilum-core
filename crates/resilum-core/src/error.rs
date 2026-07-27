use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

/// Errors surfaced by the node API.
#[derive(Debug)]
pub enum Error {
    AlreadyRunning,
    NotRunning,
    Config(String),
    /// Failure surfaced by the underlying Reticulum engine (leviculum).
    Engine(String),
    /// A routing hub was requested without an ingress policy to route through.
    VpnNoIngress,
    /// Failure bringing up or running a routing hub.
    Vpn(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::AlreadyRunning => write!(f, "node already running"),
            Error::NotRunning => write!(f, "node not running"),
            Error::Config(m) => write!(f, "config error: {m}"),
            Error::Engine(m) => write!(f, "engine error: {m}"),
            Error::VpnNoIngress => write!(f, "routing hub requires an ingress policy"),
            Error::Vpn(m) => write!(f, "routing hub error: {m}"),
        }
    }
}

impl std::error::Error for Error {}
