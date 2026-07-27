mod lifecycle;

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Mutex};

use leviculum_std::api::{Identity, Node as LevNode};
use tokio::sync::Notify;
use tokio::task::JoinHandle;

use crate::config::Config;
use crate::dispatch;
use crate::egress::CandidateRegistry;
use crate::error::{Error, Result};
use crate::event::{self, Event};
use crate::link::LinkRouter;
use crate::mirrors;

/// A Resilum node: owns the leviculum engine (with its tokio runtime) and an
/// outbound event queue.
pub struct Node {
    pub(crate) config: Config,
    pub(crate) runtime: tokio::runtime::Runtime,
    pub(crate) engine: Option<Arc<LevNode>>,
    pub(crate) registry: Arc<CandidateRegistry>,
    pub(crate) router: Option<Arc<LinkRouter>>,
    pub(crate) identity: Option<Identity>,
    pub(crate) events: dispatch::Events,
    pub(crate) tasks: Vec<JoinHandle<()>>,
    pub(crate) event_queue: event::Queue,
    pub(crate) socks_port: Arc<AtomicU16>,
    pub(crate) discovery_trigger: Arc<Notify>,
    pub(crate) mirror_registry: Option<Arc<mirrors::Registry>>,
    pub(crate) origin_registry: Arc<crate::discovery::OriginRegistry>,
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
            router: None,
            identity: None,
            events: dispatch::Events::new(1024),
            tasks: Vec::new(),
            event_queue: Arc::new(Mutex::new(VecDeque::new())),
            socks_port: Arc::new(AtomicU16::new(0)),
            discovery_trigger: Arc::new(Notify::new()),
            mirror_registry: None,
            origin_registry: Arc::new(crate::discovery::OriginRegistry::default()),
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

    /// The discovery overlay an interface was attached over (`tor` / `i2p` /
    /// `yggdrasil` / `covert`), or `None` when resilum-core did not attach it
    /// (bootstrap, LAN, or a leviculum-managed peer). Keyed by the interface id
    /// from an interface-status snapshot.
    pub fn discovered_via(&self, id: leviculum_std::InterfaceId) -> Option<String> {
        self.origin_registry.get(id)
    }

    /// Wake the discovery produce loop to re-announce endpoints now, without
    /// waiting for the next tick. Call this on external state changes the
    /// bridge cannot observe from inside (Flutter posting a network-change
    /// event through FFI, a hidden-service hostname just becoming ready, etc).
    pub fn trigger_discovery_announce(&self) {
        self.discovery_trigger.notify_waiters();
    }

    /// Attach an interface at runtime from a leviculum interface-config JSON,
    /// returning the assigned interface ids (several for a fan-out type such as
    /// RNodeMulti or an I2P interface with peers).
    pub fn add_interface(&self, config_json: &str) -> Result<Vec<u64>> {
        let engine = self.engine.as_ref().ok_or(Error::NotRunning)?;
        let config: leviculum_std::api::InterfaceConfig = serde_json::from_str(config_json)
            .map_err(|e| Error::Config(format!("interface config: {e}")))?;
        let ids = engine
            .spawn_interface(config)
            .map_err(|e| Error::Engine(e.to_string()))?;
        Ok(ids.into_iter().map(|id| id.0 as u64).collect())
    }

    /// Attach an L3 routing hub to `tun_fd`, forwarding its TCP flows through
    /// the egress mesh. Requires a running node with an ingress policy.
    #[cfg(unix)]
    pub fn vpn_attach(
        &self,
        tun_fd: std::os::fd::RawFd,
        mtu: usize,
    ) -> Result<crate::egress::vpn::VpnHandle> {
        let engine = self.engine.clone().ok_or(Error::NotRunning)?;
        let router = self.router.clone().ok_or(Error::NotRunning)?;
        let policy = self.config.ingress.clone().ok_or(Error::VpnNoIngress)?;
        let mut skip: HashMap<String, HashSet<Vec<u8>>> = HashMap::new();
        if let Some(identity) = &self.identity {
            for own in &self.config.egress {
                let hash = crate::egress::listen::dest_hash(identity.clone(), &own.service);
                skip.entry(own.service.clone()).or_default().insert(hash);
            }
        }
        let params = crate::egress::vpn::VpnParams {
            engine,
            router,
            registry: self.registry.clone(),
            policy,
            skip,
            mtu,
        };
        let _guard = self.runtime.enter();
        crate::egress::vpn::attach(params, tun_fd).map_err(|e| Error::Vpn(e.to_string()))
    }

    /// Detach an interface by id. Idempotent: an unknown id is a no-op.
    pub fn remove_interface(&self, id: u64) -> Result<()> {
        let engine = self.engine.as_ref().ok_or(Error::NotRunning)?;
        engine
            .remove_interface(leviculum_std::InterfaceId(id as usize))
            .map_err(|e| Error::Engine(e.to_string()))
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

    /// Mesh-discovered rngit mirror advertisements from peers; `None` before start.
    pub fn mirror_registry(&self) -> Option<&Arc<mirrors::Registry>> {
        self.mirror_registry.as_ref()
    }
}
