//! Shared core for a Resilum node, built on the leviculum Reticulum stack.
//! Consumed by `resilumd` and, via `resilum-ffi`, the mobile app.

pub mod announce_payload;
mod bridge;
mod config;
pub mod discovery;
pub mod dispatch;
pub mod egress;
mod engine;
mod error;
mod event;
mod identity;
pub mod link;
pub mod pump;
pub mod spec;
pub mod supervisor;

pub use config::{Config, ConnectConfig, EgressListen};
pub use error::{Error, Result};
pub use event::Event;

use std::collections::VecDeque;
use std::sync::Arc;

use leviculum_std::api::Node as LevNode;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::egress::CandidateRegistry;

/// A Resilum node: owns the leviculum engine (with its tokio runtime) and an
/// outbound event queue.
pub struct Node {
    config: Config,
    runtime: tokio::runtime::Runtime,
    engine: Option<Arc<LevNode>>,
    registry: Arc<CandidateRegistry>,
    events: dispatch::Events,
    tasks: Vec<JoinHandle<()>>,
    event_queue: VecDeque<Event>,
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
            event_queue: VecDeque::new(),
        })
    }

    pub fn start(&mut self) -> Result<()> {
        if self.engine.is_some() {
            return Err(Error::AlreadyRunning);
        }
        let (builder, identity) = engine::build_node(&self.config)?;
        let mut engine = builder.build().map_err(|e| Error::Engine(e.to_string()))?;
        // leviculum's lifecycle is async; block the caller.
        self.runtime
            .block_on(engine.start())
            .map_err(|e| Error::Engine(e.to_string()))?;
        let event_rx = engine.take_event_receiver();
        let engine = Arc::new(engine);
        let bridge_tasks = bridge::tasks_for(&self.config.specs);
        {
            let _guard = self.runtime.enter();
            let router = Arc::new(link::LinkRouter::default());
            let (inbound_tx, inbound_rx) = mpsc::unbounded_channel();
            // Subscribe every bus consumer before forward starts publishing.
            let link_bus = self.events.subscribe();
            self.tasks.push(tokio::spawn(link::run(
                router.clone(),
                link_bus,
                inbound_tx,
            )));
            if let Some(connect) = self.config.connect.clone() {
                for service in &connect.services {
                    let bus = self.events.subscribe();
                    self.tasks.push(tokio::spawn(egress::discover::run(
                        self.registry.clone(),
                        service.clone(),
                        bus,
                    )));
                }
                self.tasks.push(tokio::spawn(egress::connect::run(
                    engine.clone(),
                    router.clone(),
                    self.registry.clone(),
                    connect.clone(),
                )));
                self.tasks.push(tokio::spawn(egress::monitor::run(
                    engine.clone(),
                    router.clone(),
                    self.registry.clone(),
                    connect,
                )));
            }
            if let Some(rx) = event_rx {
                self.tasks
                    .push(tokio::spawn(dispatch::forward(self.events.clone(), rx)));
            }
            if let Some(cfg) = self.config.egress.clone() {
                self.tasks.push(tokio::spawn(egress::listen::run(
                    engine.clone(),
                    identity,
                    cfg,
                    inbound_rx,
                )));
            }
            self.tasks.extend(supervisor::spawn_all(bridge_tasks));
        }
        self.engine = Some(engine);
        self.event_queue.push_back(Event::Started);
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        for task in self.tasks.drain(..) {
            task.abort();
        }
        if let Some(engine) = self.engine.take() {
            match Arc::try_unwrap(engine) {
                Ok(mut engine) => self
                    .runtime
                    .block_on(engine.stop())
                    .map_err(|e| Error::Engine(e.to_string()))?,
                // A clone still lingers; leviculum's Drop tears the engine down.
                Err(_shared) => {}
            }
            self.event_queue.push_back(Event::Stopped);
        }
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.engine.is_some()
    }

    /// Shared engine handle for runtime tasks; `None` before start / after stop.
    pub fn engine(&self) -> Option<Arc<LevNode>> {
        self.engine.clone()
    }

    pub fn send(&mut self, _dest: &[u8], _data: &[u8]) -> Result<()> {
        if self.engine.is_none() {
            return Err(Error::NotRunning);
        }
        Ok(()) // TODO: open a Link and dispatch via egress policy.
    }

    pub fn poll_event(&mut self) -> Option<Event> {
        self.event_queue.pop_front()
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
