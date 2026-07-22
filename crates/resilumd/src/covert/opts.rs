//! Flag parsing shared by all covert carriers.

use std::path::PathBuf;

use resilum_core::covert::icmp::client::DEFAULT_MTU;

pub struct Options {
    pub dst: Option<String>,
    pub server_identity_hex: Option<String>,
    pub identity_path: Option<PathBuf>,
    pub mtu: usize,
}

pub fn parse<'a, I: Iterator<Item = &'a String>>(mut args: I) -> Result<Options, String> {
    let mut opts = Options {
        dst: None,
        server_identity_hex: None,
        identity_path: None,
        mtu: DEFAULT_MTU,
    };
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("flag {flag} needs a value"))?;
        match flag.as_str() {
            "--dst" => opts.dst = Some(value.clone()),
            "--server-identity" => opts.server_identity_hex = Some(value.clone()),
            "--identity" => opts.identity_path = Some(PathBuf::from(value)),
            "--mtu" => {
                opts.mtu = value
                    .parse::<usize>()
                    .map_err(|_| format!("bad --mtu {value}"))?
            }
            other => return Err(format!("unknown flag {other}")),
        }
    }
    Ok(opts)
}
