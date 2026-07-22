//! resilumd — thin daemon wrapping `resilum-core`: load a YAML config, start a
//! node, run until SIGINT/SIGTERM, then stop cleanly.

mod config;
mod i2pd_export;

use std::path::PathBuf;
use std::sync::mpsc;

use resilum_core::Node;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let argv: Vec<String> = std::env::args().skip(1).collect();
    match argv.first().map(String::as_str) {
        Some("generate-identity") => {
            let Some(out) = argv.get(1) else {
                eprintln!("usage: resilumd generate-identity <path>");
                std::process::exit(2);
            };
            let path = PathBuf::from(out);
            if let Some(parent) = path.parent()
                && let Err(e) = std::fs::create_dir_all(parent)
            {
                tracing::error!(path = %parent.display(), error = %e, "mkdir failed");
                std::process::exit(1);
            }
            resilum_core::identity::load_or_create_at(&path);
            tracing::info!(path = %path.display(), "identity written");
            return;
        }
        Some("i2pd-export-hostname") => {
            let (Some(keys), Some(out)) = (argv.get(1), argv.get(2)) else {
                eprintln!("usage: resilumd i2pd-export-hostname <keys.dat> <hostname-out>");
                std::process::exit(2);
            };
            std::process::exit(i2pd_export::run(keys, out));
        }
        _ => {}
    }

    let Some(path) = config_path(&argv) else {
        eprintln!("usage: resilumd [--config] <path.yaml>");
        eprintln!("       resilumd generate-identity <path>");
        std::process::exit(2);
    };

    let cfg = match config::load(&path) {
        Ok(cfg) => cfg,
        Err(e) => {
            tracing::error!(error = %e, "config load failed");
            std::process::exit(1);
        }
    };

    let mut node = match Node::new(cfg) {
        Ok(node) => node,
        Err(e) => {
            tracing::error!(error = %e, "build node failed");
            std::process::exit(1);
        }
    };
    if let Err(e) = node.start() {
        tracing::error!(error = %e, "start failed");
        std::process::exit(1);
    }
    tracing::info!(instance = node.config().instance_name.as_str(), "started");

    let (tx, rx) = mpsc::channel();
    if let Err(e) = ctrlc::set_handler(move || {
        let _ = tx.send(());
    }) {
        tracing::error!(error = %e, "signal handler install failed");
    }
    let _ = rx.recv(); // block until SIGINT/SIGTERM

    tracing::info!("stopping");
    if let Err(e) = node.stop() {
        tracing::error!(error = %e, "stop failed");
    }
}

/// `--config <path>` / `-c <path>`, or a single positional path.
fn config_path(argv: &[String]) -> Option<PathBuf> {
    match argv.first()?.as_str() {
        "--config" | "-c" => argv.get(1).map(PathBuf::from),
        positional => Some(PathBuf::from(positional)),
    }
}
