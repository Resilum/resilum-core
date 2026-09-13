use resilum_core::Node;

pub(super) fn log_startup(node: &Node) {
    let id_hash = node
        .engine()
        .as_ref()
        .map(|e| e.identity_hash())
        .map(|h| resilum_core::hex::encode(h.iter().take(8)))
        .unwrap_or_default();
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        instance = node.config().instance_name.as_str(),
        identity = %id_hash,
        "started"
    );
}

pub(super) fn spawn_health(node: &Node) {
    let Some(engine) = node.engine() else {
        return;
    };
    let path = crate::health::file_path(
        node.config().storage_path.as_deref(),
        std::env::var("RESILUM_HEALTH_FILE").ok(),
    );
    crate::health::spawn(engine, node.tasks(), path);
}

pub(super) fn spawn_stats(node: &Node) {
    let Some(engine) = node.engine() else {
        return;
    };
    let started = resilum_tasks::a_thread_of_its_own("what the transport carried", move || {
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
    if let Err(e) = started {
        tracing::warn!(error = %e, "no thread to report transport stats from");
    }
}
