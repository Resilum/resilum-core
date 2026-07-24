use std::fmt;

#[derive(Debug)]
pub enum SocksError {
    LinkClosed,
    BadVersion(u8),
    NoAcceptableAuth,
    UnsupportedCommand(u8),
    UnsupportedAtyp(u8),
    BadDomain,
}

impl fmt::Display for SocksError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SocksError::LinkClosed => write!(f, "peer closed link during handshake"),
            SocksError::BadVersion(v) => write!(f, "unsupported SOCKS version {v:#x}"),
            SocksError::NoAcceptableAuth => write!(f, "client offered no acceptable auth method"),
            SocksError::UnsupportedCommand(c) => write!(f, "unsupported SOCKS command {c:#x}"),
            SocksError::UnsupportedAtyp(a) => write!(f, "unsupported address type {a:#x}"),
            SocksError::BadDomain => write!(f, "invalid domain name in CONNECT request"),
        }
    }
}

impl std::error::Error for SocksError {}
