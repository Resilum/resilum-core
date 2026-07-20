//! resilum-core — shared core for a Resilum node.
//!
//! Single source of truth consumed by the `resilumd` daemon and by the mobile
//! app (through `resilum-ffi`). Built on the leviculum Rust Reticulum stack
//! (ADR-003). This first slice mirrors the project's node-level interface
//! bringup (`network config` `[interfaces]`): `Config` → a leviculum
//! `NodeBuilder`. Discovery, egress policy and the service bridges follow.

mod config;
mod error;
mod event;

pub use config::Config;
pub use error::{Error, Result};
pub use event::Event;

use std::collections::VecDeque;

use leviculum_std::api::{self, Node as LevNode, NodeBuilder};

/// A Resilum node. Owns the leviculum engine (with its tokio runtime) and an
/// outbound event queue.
pub struct Node {
    config: Config,
    runtime: tokio::runtime::Runtime,
    engine: Option<LevNode>,
    events: VecDeque<Event>,
}

/// Translate `Config` into a configured leviculum `NodeBuilder`.
///
/// This is the analog of the project rendering `network config`'s
/// `[interfaces]` block: a public TCP listener, TCP clients to bootstrap
/// anchors, and the local-segment `AutoInterface`.
fn configure_builder(config: &Config) -> Result<NodeBuilder> {
    // TODO: load-or-generate a stable identity from `storage_path` instead of a
    // fresh one each start (mirror the `network_identity`).
    let mut builder = NodeBuilder::new().identity(api::generate_identity());

    if let Some(path) = &config.storage_path {
        builder = builder.storage_path(path.clone());
    }

    if let Some(listen) = &config.listen {
        let addr = listen
            .parse()
            .map_err(|_| Error::Config(format!("bad listen address: {listen}")))?;
        builder = builder.add_tcp_server(addr);
    }

    for anchor in &config.bootstrap {
        let addr = anchor
            .parse()
            .map_err(|_| Error::Config(format!("bad bootstrap address: {anchor}")))?;
        builder = builder.add_tcp_client(addr);
    }

    if config.discover_interfaces {
        builder = builder.add_auto_interface();
    }

    Ok(builder)
}

impl Node {
    /// Build a node from config. Does not start any I/O.
    pub fn new(config: Config) -> Result<Self> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|e| Error::Engine(format!("tokio runtime: {e}")))?;
        Ok(Self {
            config,
            runtime,
            engine: None,
            events: VecDeque::new(),
        })
    }

    /// Start the node: build the leviculum engine from config and bring up
    /// interfaces.
    pub fn start(&mut self) -> Result<()> {
        if self.engine.is_some() {
            return Err(Error::AlreadyRunning);
        }
        let mut engine = configure_builder(&self.config)?
            .build()
            .map_err(|e| Error::Engine(e.to_string()))?;
        self.runtime
            .block_on(engine.start())
            .map_err(|e| Error::Engine(e.to_string()))?;
        self.engine = Some(engine);
        self.events.push_back(Event::Started);
        Ok(())
    }

    /// Stop the node and tear down the engine.
    pub fn stop(&mut self) -> Result<()> {
        if let Some(mut engine) = self.engine.take() {
            self.runtime
                .block_on(engine.stop())
                .map_err(|e| Error::Engine(e.to_string()))?;
            self.events.push_back(Event::Stopped);
        }
        Ok(())
    }

    /// Whether the node is running.
    pub fn is_running(&self) -> bool {
        self.engine.is_some()
    }

    /// Send `data` to a destination hash.
    pub fn send(&mut self, _dest: &[u8], _data: &[u8]) -> Result<()> {
        if self.engine.is_none() {
            return Err(Error::NotRunning);
        }
        // TODO: resolve a path / open a Link and dispatch via egress policy.
        Ok(())
    }

    /// Pop the next queued event, if any. The daemon and FFI poll this.
    pub fn poll_event(&mut self) -> Option<Event> {
        self.events.pop_front()
    }

    /// The config the node was built with.
    pub fn config(&self) -> &Config {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_maps_to_a_builder() {
        // A well-formed config configures a builder without error.
        let cfg = Config {
            instance_name: "test".into(),
            storage_path: None,
            listen: Some("[::]:4242".into()),
            bootstrap: vec!["203.0.113.10:4242".into()],
            discover_interfaces: true,
        };
        assert!(configure_builder(&cfg).is_ok());
    }

    #[test]
    fn bad_listen_address_is_a_config_error() {
        let cfg = Config {
            listen: Some("not-an-address".into()),
            ..Config::minimal("test")
        };
        assert!(matches!(configure_builder(&cfg), Err(Error::Config(_))));
    }

    #[test]
    fn bad_bootstrap_address_is_a_config_error() {
        let cfg = Config {
            bootstrap: vec!["nope".into()],
            ..Config::minimal("test")
        };
        assert!(matches!(configure_builder(&cfg), Err(Error::Config(_))));
    }
}
