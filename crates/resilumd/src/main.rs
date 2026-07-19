//! resilumd — thin daemon wrapping `resilum-core`.
//!
//! The Linux/server frontend; the container image packages this binary.
//! Skeleton: builds a node, starts it, drains queued events, stops. Config
//! loading and signal handling are TODO.

use resilum_core::{Config, Event, Node};

fn main() {
    // TODO: load config from a file/env instead of this placeholder.
    let config = Config::minimal("resilumd");

    let mut node = match Node::new(config) {
        Ok(node) => node,
        Err(e) => {
            eprintln!("[resilumd] failed to build node: {e}");
            std::process::exit(1);
        }
    };

    if let Err(e) = node.start() {
        eprintln!("[resilumd] failed to start: {e}");
        std::process::exit(1);
    }
    println!("[resilumd] started (skeleton — no transports yet)");

    // TODO: replace with a real run loop + signal handling (SIGINT/SIGTERM).
    while let Some(event) = node.poll_event() {
        log_event(&event);
    }

    let _ = node.stop();
}

fn log_event(event: &Event) {
    match event {
        Event::Started => println!("[resilumd] event: started"),
        Event::Stopped => println!("[resilumd] event: stopped"),
        Event::PeerDiscovered(hash) => {
            println!("[resilumd] event: peer discovered ({} bytes)", hash.len());
        }
        Event::Received { source, data } => {
            println!(
                "[resilumd] event: received {} bytes from a {}-byte hash",
                data.len(),
                source.len()
            );
        }
    }
}
