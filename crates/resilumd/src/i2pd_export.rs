//! Wait for i2pd to publish its server-tunnel keys, then write the matching
//! `.b32.i2p` hostname to a separate file.
//!
//! The .b32.i2p hostname is `base32(sha256(first 391 bytes of keys.dat))` —
//! the 391-byte prefix is the public key (384) plus a 7-byte certificate for
//! the default EdDSA-25519 signature type.

use std::path::Path;
use std::time::{Duration, Instant};

use data_encoding::BASE32_NOPAD;
use sha2::{Digest, Sha256};

const PREFIX_BYTES: usize = 391;
const WAIT_TIMEOUT: Duration = Duration::from_secs(600);
const POLL: Duration = Duration::from_secs(2);

pub fn run(keys_path: &str, hostname_path: &str) -> i32 {
    let hostname_path = Path::new(hostname_path);
    if hostname_path.exists() {
        return 0;
    }
    if !wait_for_keys(Path::new(keys_path)) {
        eprintln!(
            "[i2pd-export] {keys_path} did not appear within {}s",
            WAIT_TIMEOUT.as_secs()
        );
        return 1;
    }
    match derive(Path::new(keys_path)).and_then(|hn| write_hostname(hostname_path, &hn)) {
        Ok(hn) => {
            tracing::info!(hostname = %hn, path = %hostname_path.display(), "written");
            0
        }
        Err(e) => {
            eprintln!("[i2pd-export] {e}");
            1
        }
    }
}

fn wait_for_keys(path: &Path) -> bool {
    let deadline = Instant::now() + WAIT_TIMEOUT;
    while Instant::now() < deadline {
        if let Ok(md) = std::fs::metadata(path)
            && md.len() as usize >= PREFIX_BYTES
        {
            return true;
        }
        std::thread::sleep(POLL);
    }
    false
}

fn derive(keys_path: &Path) -> Result<String, String> {
    let bytes =
        std::fs::read(keys_path).map_err(|e| format!("read {}: {e}", keys_path.display()))?;
    if bytes.len() < PREFIX_BYTES {
        return Err(format!(
            "{} has {} bytes, expected at least {PREFIX_BYTES}",
            keys_path.display(),
            bytes.len()
        ));
    }
    let digest = Sha256::digest(&bytes[..PREFIX_BYTES]);
    let b32 = BASE32_NOPAD.encode(&digest).to_lowercase();
    Ok(format!("{b32}.b32.i2p"))
}

fn write_hostname(path: &Path, hostname: &str) -> Result<String, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
    }
    std::fs::write(path, format!("{hostname}\n"))
        .map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(hostname.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_known_hostname() {
        let dir = std::env::temp_dir().join(format!("resilumd-i2pd-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let keys = dir.join("keys.dat");
        std::fs::write(&keys, vec![0x42u8; PREFIX_BYTES]).unwrap();
        let hn = derive(&keys).unwrap();
        assert!(hn.ends_with(".b32.i2p"));
        let label = hn.strip_suffix(".b32.i2p").unwrap();
        // base32 of a 32-byte sha256 is 52 characters (no padding).
        assert_eq!(label.len(), 52);
        assert!(
            label
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
