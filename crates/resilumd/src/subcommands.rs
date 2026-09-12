//! The one-shot verbs, before the daemon path is considered.

use std::path::PathBuf;

use crate::{bind_config, covert, i2pd_export, mirrors, ygg_seed};

pub fn dispatch_or_exit(argv: &[String]) {
    match argv.first().map(String::as_str) {
        Some("generate-identity") => generate_identity(argv),
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
        Some("status") => std::process::exit(crate::status::run(&argv[1..])),
        Some("covert") => std::process::exit(covert::run(&argv[1..])),
        Some("mirrors") => std::process::exit(mirrors::run(&argv[1..])),
        Some("probe-net") => probe_net(),
        _ => {}
    }
}

fn generate_identity(argv: &[String]) -> ! {
    let Some(out) = argv.get(1) else {
        eprintln!("usage: resilumd generate-identity <path>");
        std::process::exit(2);
    };
    let path = PathBuf::from(out);
    if let Some(parent) = path.parent()
        && let Err(e) = resilum_store::make_room_for(parent)
    {
        tracing::error!(path = %parent.display(), error = %e, "mkdir failed");
        std::process::exit(1);
    }
    resilum_core::identity::load_or_create_at(&path);
    tracing::info!(path = %path.display(), "identity written");
    std::process::exit(0);
}

fn probe_net() -> ! {
    let ygg = resilum_core::net::yggdrasil_local_ipv6();
    println!("yggdrasil_local_ipv6 = {ygg:?}");
    for i in if_addrs::get_if_addrs().unwrap_or_default() {
        println!("  {} → {}", i.name, i.ip());
    }
    std::process::exit(0);
}

pub fn usage() -> ! {
    eprintln!("usage: resilumd [--config] <path.yaml>");
    eprintln!("       resilumd generate-identity <path>");
    eprintln!("       resilumd i2pd-export-hostname <keys.dat> <hostname-out>");
    eprintln!("       resilumd ygg-seed-keys <yggdrasil.conf>");
    eprintln!("       resilumd render-bind-config [<ygg.conf>] [<rns.conf>]");
    eprintln!("       resilumd status [<path.yaml>] [--interfaces] [--links] [--map]");
    eprintln!("                       [--all] [--color] [--json]");
    eprintln!("       resilumd covert <carrier> <client|server> [flags]");
    std::process::exit(2);
}

/// `--config <path>` / `-c <path>`, or a single positional path.
pub fn config_path(argv: &[String]) -> Option<PathBuf> {
    match argv.first()?.as_str() {
        "--config" | "-c" => argv.get(1).map(PathBuf::from),
        positional => Some(PathBuf::from(positional)),
    }
}
