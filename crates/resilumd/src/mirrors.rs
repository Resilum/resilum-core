//! `resilumd mirrors find <repo>`: print rns URLs of mesh peers advertising a
//! rngit mirror of `<repo>`. Reads the registry file the running daemon
//! persists under `storage_path/mirror_registry.json`.

use std::path::Path;

use serde::Deserialize;

const USAGE: &str = "usage: resilumd mirrors find <repo> [--registry <path>]";

pub fn run(argv: &[String]) -> i32 {
    let mut args = argv.iter();
    let Some(subcmd) = args.next() else {
        crate::out::refused(USAGE);
        return 2;
    };
    if subcmd != "find" {
        crate::out::refused(format_args!("unknown mirrors subcommand: {subcmd}"));
        return 2;
    }
    let Some(repo) = args.next() else {
        crate::out::refused(USAGE);
        return 2;
    };
    let mut path = default_registry_path();
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--registry" => path = args.next().map(std::path::PathBuf::from),
            _ => {
                crate::out::refused(format_args!("unknown flag: {flag}"));
                return 2;
            }
        }
    }
    let Some(path) = path else {
        crate::out::refused("no --registry supplied and no default storage path found");
        return 2;
    };
    match load(&path) {
        Ok(entries) => {
            for e in entries.iter().filter(|e| e.repos.iter().any(|r| r == repo)) {
                crate::out::shown_as_a_line(format_args!("rns://{}/mirrors/{}", e.rngit, repo));
            }
            0
        }
        Err(e) => {
            crate::out::refused(format_args!("registry read {}: {e}", path.display()));
            1
        }
    }
}

fn default_registry_path() -> Option<std::path::PathBuf> {
    let candidates = ["/config/state/mirror_registry.json"];
    candidates
        .iter()
        .map(std::path::PathBuf::from)
        .find(|p| p.exists())
}

#[derive(Deserialize)]
struct Entry {
    rngit: String,
    repos: Vec<String>,
}

fn load(path: &Path) -> std::io::Result<Vec<Entry>> {
    let bytes = resilum_store::read_bytes(path)?;
    serde_json::from_slice(&bytes)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}
