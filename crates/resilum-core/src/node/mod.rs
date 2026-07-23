mod lifecycle;

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Mutex};

use leviculum_std::api::Node as LevNode;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

use crate::config::Config;
use crate::dispatch;
use crate::egress::CandidateRegistry;
use crate::error::{Error, Result};
use crate::event::{self, Event};

/// A Resilum node: owns the leviculum engine (with its tokio runtime) and an
/// outbound event queue.
pub struct Node {
    pub(crate) config: Config,
    pub(crate) runtime: tokio::runtime::Runtime,
    pub(crate) engine: Option<Arc<LevNode>>,
    pub(crate) registry: Arc<CandidateRegistry>,
    pub(crate) events: dispatch::Events,
    pub(crate) tasks: Vec<JoinHandle<()>>,
    pub(crate) event_queue: event::Queue,
    pub(crate) socks_port: Arc<AtomicU16>,
    pub(crate) discovery_trigger: Arc<Notify>,
    #[cfg(feature = "arti")]
    pub(crate) embedded_tor: Option<crate::tor::EmbeddedTor>,
}

impl Node {
    pub fn new(config: Config) -> Result<Self> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|e| Error::Engine(format!("tokio runtime: {e}")))?;
        Ok(Self {
            config,
            runtime,
            engine: None,
            registry: Arc::new(CandidateRegistry::default()),
            events: dispatch::Events::new(1024),
            tasks: Vec::new(),
            event_queue: Arc::new(Mutex::new(VecDeque::new())),
            socks_port: Arc::new(AtomicU16::new(0)),
            discovery_trigger: Arc::new(Notify::new()),
            #[cfg(feature = "arti")]
            embedded_tor: None,
        })
    }

    pub fn is_running(&self) -> bool {
        self.engine.is_some()
    }

    /// The bound local SOCKS/connect port, or `0` if the connect listener is not
    /// up (no connect config, or not yet bound).
    pub fn socks_port(&self) -> u16 {
        self.socks_port.load(Ordering::Relaxed)
    }

    /// Shared engine handle for runtime tasks; `None` before start / after stop.
    pub fn engine(&self) -> Option<Arc<LevNode>> {
        self.engine.clone()
    }

    /// Wake the discovery produce loop to re-announce endpoints now, without
    /// waiting for the next tick. Call this on external state changes the
    /// bridge cannot observe from inside (Flutter posting a network-change
    /// event through FFI, a hidden-service hostname just becoming ready, etc).
    pub fn trigger_discovery_announce(&self) {
        self.discovery_trigger.notify_waiters();
    }

    pub fn send(&mut self, _dest: &[u8], _data: &[u8]) -> Result<()> {
        if self.engine.is_none() {
            return Err(Error::NotRunning);
        }
        Ok(())
    }

    pub fn poll_event(&mut self) -> Option<Event> {
        self.event_queue.lock().expect("event queue").pop_front()
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    /// The shared egress candidate registry.
    pub fn registry(&self) -> &Arc<CandidateRegistry> {
        &self.registry
    }

    /// The node-event bus; subsystems subscribe to receive engine events.
    pub fn events(&self) -> &dispatch::Events {
        &self.events
    }
}
