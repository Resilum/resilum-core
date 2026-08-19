use std::path::Path;
use std::sync::mpsc;

use resilum_core::Node;
use resilum_core::discovery::Service;

/// The shutdown path returns rather than exiting, so `_iroh` and the node are
/// dropped: the iroh handle's teardown sends CONNECTION_CLOSE, which a
/// `process::exit` would skip. The startup failures below do exit, having
/// nothing yet to tear down.
pub fn run(path: &Path) {
    let mut cfg = match crate::config::load(path) {
        Ok(cfg) => cfg,
        Err(e) => {
            tracing::error!(error = %e, "config load failed");
            std::process::exit(1);
        }
    };
    // Read once, before the node exists: `discovery::bring_up` needs to know
    // at `node.start()` time whether a bridge will publish, since the bridge
    // itself only starts (and is advertised) afterwards.
    let nostr_cfg = crate::nostr::load(path);
    if nostr_cfg.as_ref().is_some_and(|c| c.publish) {
        cfg.advertised_services.push(Service::NOSTR_RELAY);
    }
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
    // The daemon binds its own iroh socket, so it attaches itself rather than
    // waiting to be driven over the FFI. No socket protection here: the daemon
    // is not under a captured tun, so it uses iroh's stock transports.
    let _iroh = attach_iroh(&mut node);
    let _nostr = nostr_cfg.and_then(|cfg| {
        let publish = cfg.publish;
        let handle = crate::nostr::start(&node, cfg)?;
        if publish {
            crate::nostr::advertise_relay(&node);
        }
        Some(handle)
    });
    log_startup(&node);
    spawn_stats(&node);
    spawn_health(&node);

    let (tx, rx) = mpsc::channel();
    if let Err(e) = ctrlc::set_handler(move || {
        let _ = tx.send(());
    }) {
        // `set_handler` consumed `tx` and dropped it, so `rx.recv()` would
        // return immediately and the daemon would shut down looking clean
        // seconds after start. There is no signal to wait for; fail loudly.
        tracing::error!(error = %e, "signal handler install failed");
        std::process::exit(1);
    }
    let _ = rx.recv();

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

fn log_startup(node: &Node) {
    let id_hash = node
        .engine()
        .as_ref()
        .map(|e| e.identity_hash())
        .map(|h| resilum_core::hex::encode(h.iter().take(8)))
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
