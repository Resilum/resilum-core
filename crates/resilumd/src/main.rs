//! resilumd — thin daemon wrapping `resilum-core`: load a YAML config, start a
//! node, run until SIGINT/SIGTERM, then stop cleanly.

mod bind_config;
mod config;
mod covert;
mod daemon;
mod i2pd_export;
mod mirrors;
mod subcommands;
mod ygg_seed;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let argv: Vec<String> = std::env::args().skip(1).collect();
    subcommands::dispatch(&argv);

    match subcommands::config_path(&argv) {
        Some(path) => daemon::run(&path),
        None => subcommands::usage(),
    }
}
