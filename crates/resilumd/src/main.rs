//! resilumd — thin daemon wrapping `resilum-core`: load a YAML config, start a
//! node, run until SIGINT/SIGTERM, then stop cleanly.

mod config;

use std::path::PathBuf;
use std::sync::mpsc;

use resilum_core::Node;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let Some(path) = config_path() else {
        eprintln!("usage: resilumd --config <path.yaml>");
        std::process::exit(2);
    };

    let cfg = match config::load(&path) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("[resilumd] config error: {e}");
            std::process::exit(1);
        }
    };

    let mut node = match Node::new(cfg) {
        Ok(node) => node,
        Err(e) => {
            eprintln!("[resilumd] build node: {e}");
            std::process::exit(1);
        }
    };
    if let Err(e) = node.start() {
        eprintln!("[resilumd] start: {e}");
        std::process::exit(1);
    }
    tracing::info!(instance = node.config().instance_name.as_str(), "started");

    let (tx, rx) = mpsc::channel();
    if let Err(e) = ctrlc::set_handler(move || {
        let _ = tx.send(());
    }) {
        eprintln!("[resilumd] signal handler: {e}");
    }
    let _ = rx.recv(); // block until SIGINT/SIGTERM

    tracing::info!("stopping");
    if let Err(e) = node.stop() {
        eprintln!("[resilumd] stop: {e}");
    }
}

/// `--config <path>` / `-c <path>`, or a single positional path.
fn config_path() -> Option<PathBuf> {
    let mut args = std::env::args().skip(1);
    match args.next()?.as_str() {
        "--config" | "-c" => args.next().map(PathBuf::from),
        positional => Some(PathBuf::from(positional)),
    }
}
