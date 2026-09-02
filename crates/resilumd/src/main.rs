//! resilumd — thin daemon wrapping `resilum-core`: load a YAML config, start a
//! node, run until SIGINT/SIGTERM, then stop cleanly.

mod bind_config;
mod config;
mod covert;
mod daemon;
mod health;
mod i2pd_export;
mod mirrors;
mod nostr;
mod radio_facts;
mod subcommands;
mod wifi_group;
mod ygg_seed;

fn main() {
    // `from_default_env` alone leaves an unset `RUST_LOG` at ERROR, and a
    // daemon that starts successfully then prints nothing looks hung.
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let argv: Vec<String> = std::env::args().skip(1).collect();
    subcommands::dispatch_or_exit(&argv);

    match subcommands::config_path(&argv) {
        Some(path) => daemon::run(&path),
        None => subcommands::usage(),
    }
}
