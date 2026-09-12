//! `resilumd mirrors find <repo>`: print rns URLs of mesh peers advertising a
//! rngit mirror of `<repo>`. Reads the registry file the running daemon
//! persists under `storage_path/mirror_registry.json`.

use std::path::Path;

use serde::Deserialize;

pub fn run(argv: &[String]) -> i32 {
    let mut args = argv.iter();
    let Some(subcmd) = args.next() else {
        eprintln!("usage: resilumd mirrors find <repo> [--registry <path>]");
        return 2;
    };
    if subcmd != "find" {
        eprintln!("unknown mirrors subcommand: {subcmd}");
        return 2;
    }
    let Some(repo) = args.next() else {
        eprintln!("usage: resilumd mirrors find <repo> [--registry <path>]");
        return 2;
    };
    let mut path = default_registry_path();
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--registry" => path = args.next().map(std::path::PathBuf::from),
            _ => {
                eprintln!("unknown flag: {flag}");
                return 2;
            }
        }
    }
    let Some(path) = path else {
        eprintln!("no --registry supplied and no default storage path found");
        return 2;
    };
    match load(&path) {
        Ok(entries) => {
            for e in entries.iter().filter(|e| e.repos.iter().any(|r| r == repo)) {
                println!("rns://{}/mirrors/{}", e.rngit, repo);
            }
            0
        }
        Err(e) => {
            eprintln!("registry read {}: {e}", path.display());
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
