//! `resilumd covert <carrier> <client|server> ...` subcommand.
//! Spawned by a leviculum PipeInterface to bridge one covert link.

mod hex;
mod icmp;
mod opts;
mod stdio;

const USAGE: &str = "usage: resilumd covert <carrier> <client|server> [flags]";

pub fn run(argv: &[String]) -> i32 {
    let mut args = argv.iter();
    let Some(carrier) = args.next() else {
        eprintln!("{USAGE}");
        return 2;
    };
    let Some(role) = args.next() else {
        eprintln!("{USAGE}");
        return 2;
    };
    let options = match opts::parse(args) {
        Ok(o) => o,
        Err(msg) => {
            eprintln!("{msg}");
            return 2;
        }
    };
    match (carrier.as_str(), role.as_str()) {
        ("icmp", "client") => icmp::client(options),
        ("icmp", "server") => icmp::server(options),
        _ => {
            eprintln!("unsupported carrier/role: {carrier} {role}");
            2
        }
    }
}
