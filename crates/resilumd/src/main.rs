//! resilumd — thin daemon wrapping `resilum-core`: load a YAML config, start a
//! node, run until SIGINT/SIGTERM, then stop cleanly.

mod bind_config;
mod config;
mod covert;
mod i2pd_export;
mod mirrors;
mod ygg_seed;

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
        Some("ygg-seed-keys") => {
            let Some(cfg) = argv.get(1) else {
                eprintln!("usage: resilumd ygg-seed-keys <yggdrasil.conf>");
                std::process::exit(2);
            };
            std::process::exit(ygg_seed::run(cfg));
        }
        Some("render-bind-config") => {
            std::process::exit(bind_config::run(
                argv.get(1).map(String::as_str),
                argv.get(2).map(String::as_str),
            ));
        }
        Some("covert") => {
            std::process::exit(covert::run(&argv[1..]));
        }
        Some("mirrors") => {
            std::process::exit(mirrors::run(&argv[1..]));
        }
        Some("probe-net") => {
            let ygg = resilum_core::net::yggdrasil_local_ipv6();
            println!("yggdrasil_local_ipv6 = {ygg:?}");
            for i in if_addrs::get_if_addrs().unwrap_or_default() {
                println!("  {} → {}", i.name, i.ip());
            }
            return;
        }
        _ => {}
    }

    let Some(path) = config_path(&argv) else {
        eprintln!("usage: resilumd [--config] <path.yaml>");
        eprintln!("       resilumd generate-identity <path>");
        eprintln!("       resilumd i2pd-export-hostname <keys.dat> <hostname-out>");
        eprintln!("       resilumd ygg-seed-keys <yggdrasil.conf>");
        eprintln!("       resilumd render-bind-config [<ygg.conf>] [<rns.conf>]");
        eprintln!("       resilumd covert <carrier> <client|server> [flags]");
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
    // Server binds its own iroh socket, so it attaches itself (no app to drive
    // the FFI). Held until stop; dropping detaches. No socket-protect here — the
    // server isn't under a captured tun, so it uses iroh's stock transports.
    let _iroh = if node.config().iroh.is_some() {
        match node.iroh_attach() {
            Ok(handle) => {
                tracing::info!(endpoint_id = %handle.endpoint_id(), "iroh attached");
                Some(handle)
            }
            Err(e) => {
                tracing::error!(error = %e, "iroh attach failed");
                None
            }
        }
    } else {
        None
    };
    let id_hash = node
        .engine()
        .as_ref()
        .map(|e| e.identity_hash())
        .map(|h| {
            h.iter()
                .take(8)
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        })
        .unwrap_or_default();
    tracing::info!(
        instance = node.config().instance_name.as_str(),
        identity = %id_hash,
        "started"
    );

    if let Some(engine) = node.engine() {
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_secs(30));
                let s = engine.transport_stats();
                tracing::info!(
                    sent = s.packets_sent(),
                    recv = s.packets_received(),
                    forwarded = s.packets_forwarded(),
                    announces = s.announces_processed(),
                    paths = engine.path_count(),
                    "transport stats"
                );
            }
        });
    }

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
