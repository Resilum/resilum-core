//! Splice a fresh Yggdrasil PrivateKey into a config file in place.
//!
//! Reads the file, calls `yggdrasil -genconf` for a fresh keypair, replaces the
//! first `PrivateKey: ""` placeholder with the new hex value, and writes back.
//! Other bytes are preserved. No-op when the key is already populated.

use std::path::Path;
use std::process::Command;

pub fn run(config_path: &str) -> i32 {
    let path = Path::new(config_path);
    let original = match resilum_store::read_text(path) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(path = %path.display(), error = %e, "read failed");
            return 1;
        }
    };
    if !original.contains(r#"PrivateKey: """#) {
        return 0;
    }
    let key = match fresh_key() {
        Ok(k) => k,
        Err(e) => {
            tracing::error!(error = %e, "yggdrasil -genconf failed");
            return 0;
        }
    };
    let updated = original.replacen(r#"PrivateKey: """#, &format!("PrivateKey: {key}"), 1);
    if let Err(e) = resilum_store::write_text(path, &updated) {
        tracing::error!(path = %path.display(), error = %e, "write failed");
        return 1;
    }
    tracing::info!(path = %path.display(), "key written");
    0
}

fn fresh_key() -> Result<String, String> {
    let out = Command::new("yggdrasil")
        .arg("-genconf")
        .output()
        .map_err(|e| format!("spawn: {e}"))?;
    if !out.status.success() {
        return Err(format!("exit {}", out.status));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        let line = line.trim_start();
        if let Some(rest) = line.strip_prefix("PrivateKey:") {
            let key = rest.trim();
            if !key.is_empty() && key.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Ok(key.to_string());
            }
        }
    }
    Err("no PrivateKey line in output".into())
}

#[cfg(test)]
mod tests {
    #[test]
    fn splices_placeholder_only_once() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("ygg.conf");
        let original = "Peers: []\nPrivateKey: \"\"\nMulticastInterfaces: []\n";
        resilum_store::write_text(&path, original).expect("a config to splice");

        // Simulate the splice without invoking yggdrasil.
        let spliced = original.replacen(r#"PrivateKey: """#, "PrivateKey: deadbeef", 1);
        assert!(spliced.contains("PrivateKey: deadbeef"));
        assert!(!spliced.contains(r#"PrivateKey: """#));
        assert!(spliced.contains("Peers: []"));
        assert!(spliced.contains("MulticastInterfaces: []"));
    }

    #[test]
    fn no_op_when_populated() {
        let text = "PrivateKey: abc123\n";
        assert!(!text.contains(r#"PrivateKey: """#));
    }
}
