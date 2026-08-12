//! The daemon proper: start a node, report on it, stop on a signal.

use std::path::Path;
use std::sync::mpsc;

use resilum_core::Node;

/// Returns rather than exiting, so `_iroh` and the node are dropped: the iroh
/// handle's teardown sends CONNECTION_CLOSE, which a `process::exit` would skip.
pub fn run(path: &Path) {
    let cfg = match crate::config::load(path) {
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
    // The server binds its own iroh socket, so it attaches itself — there is no
    // app to drive the FFI. Held until stop; dropping detaches. No socket
    // protection here: the server is not under a captured tun, so it uses
    // iroh's stock transports.
    let _iroh = attach_iroh(&mut node);
    announce_startup(&node);
    spawn_stats(&node);
    spawn_health(&node);

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

fn attach_iroh(node: &mut Node) -> Option<resilum_core::IrohHandle> {
    node.config().iroh.as_ref()?;
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
}

fn announce_startup(node: &Node) {
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
}

fn spawn_health(node: &Node) {
    let Some(engine) = node.engine() else {
        return;
    };
    let path = crate::health::file_path(
        node.config().storage_path.as_deref(),
        std::env::var("RESILUM_HEALTH_FILE").ok(),
    );
    crate::health::spawn(engine, path);
}

fn spawn_stats(node: &Node) {
    let Some(engine) = node.engine() else {
        return;
    };
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
