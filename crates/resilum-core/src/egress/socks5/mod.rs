//! Server-side SOCKS5 CONNECT for the embedded egress backend (RFC 1928,
//! no-auth). UDP ASSOCIATE and BIND are not implemented.

mod client;
mod error;
mod parse;

#[cfg(test)]
mod tests;

pub use client::{Target, connect};
pub use error::SocksError;
pub use parse::handshake;

pub(crate) const VER: u8 = 0x05;
const METHOD_NO_AUTH: u8 = 0x00;
const CMD_CONNECT: u8 = 0x01;
const ATYP_V4: u8 = 0x01;
const ATYP_DOMAIN: u8 = 0x03;
const ATYP_V6: u8 = 0x04;

pub const AUTH_NO_AUTH: [u8; 2] = [VER, METHOD_NO_AUTH];
pub const REPLY_OK: [u8; 10] = [VER, 0x00, 0x00, ATYP_V4, 0, 0, 0, 0, 0, 0];
pub const REPLY_HOST_UNREACHABLE: [u8; 10] = [VER, 0x04, 0x00, ATYP_V4, 0, 0, 0, 0, 0, 0];
