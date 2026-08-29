//! ICMP client/server binding for the covert subcommand.

use std::io;
use std::net::IpAddr;

use resilum_core::covert::icmp::client::IcmpClient;
use resilum_core::covert::icmp::server::IcmpServer;
use resilum_core::identity;

use super::hex;
use super::opts::Options;
use super::stdio;

pub fn client(opts: Options) -> i32 {
    let Some(dst) = opts.dst else {
        eprintln!("--dst required for client");
        return 2;
    };
    let Some(hex_str) = opts.server_identity_hex else {
        eprintln!("--server-identity required for client");
        return 2;
    };
    let server = match hex::identity(&hex_str) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("bad server identity: {e}");
            return 2;
        }
    };
    let addr: IpAddr = match dst.parse() {
        Ok(a) => a,
        Err(_) => {
            eprintln!("--dst must be an IP address");
            return 2;
        }
    };
    let client = match IcmpClient::with_mtu(addr, &server.public_key_bytes(), opts.mtu) {
        Ok(c) => c,
        Err(e) => return exit_io("open icmp socket", e),
    };
    match stdio::client(client, server, random_session_id()) {
        Ok(()) => 0,
        Err(e) => exit_io("client driver", e),
    }
}

pub fn server(opts: Options) -> i32 {
    let Some(path) = opts.identity_path else {
        eprintln!("--identity required for server");
        return 2;
    };
    let identity = identity::load_or_create_at(&path);
    let srv = match IcmpServer::with_mtu(&identity.public_key_bytes(), opts.mtu) {
        Ok(s) => s,
        Err(e) => return exit_io("open icmp server", e),
    };
    match stdio::server(srv, identity) {
        Ok(()) => 0,
        Err(e) => exit_io("server driver", e),
    }
}

fn random_session_id() -> u32 {
    use rand_core::RngCore;
    let mut buf = [0u8; 4];
    rand_core::OsRng.fill_bytes(&mut buf);
    u32::from_be_bytes(buf)
}

fn exit_io(what: &str, e: io::Error) -> i32 {
    tracing::error!(what = what, error = %e, "covert bridge failed");
    1
}
