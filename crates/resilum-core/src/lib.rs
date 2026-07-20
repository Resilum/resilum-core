//! Shared core for a Resilum node, built on the leviculum Reticulum stack.
//! Consumed by `resilumd` and, via `resilum-ffi`, the mobile app.

mod bridge;
mod config;
pub mod discovery;
pub mod dispatch;
pub mod egress;
mod engine;
mod error;
mod event;
pub mod spec;
pub mod supervisor;

pub use config::Config;
pub use error::{Error, Result};
pub use event::Event;

use std::collections::VecDeque;
use std::sync::Arc;

use leviculum_std::api::Node as LevNode;
use tokio::task::JoinHandle;

use crate::egress::CandidateRegistry;

/// A Resilum node: owns the leviculum engine (with its tokio runtime) and an
/// outbound event queue.
pub struct Node {
    config: Config,
    runtime: tokio::runtime::Runtime,
    engine: Option<LevNode>,
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
        let mut engine = engine::build_node(&self.config)?
            .build()
            .map_err(|e| Error::Engine(e.to_string()))?;
        // leviculum's lifecycle is async; block the caller.
        self.runtime
            .block_on(engine.start())
            .map_err(|e| Error::Engine(e.to_string()))?;
        let event_rx = engine.take_event_receiver();
        let bridge_tasks = bridge::tasks_for(&self.config.specs);
        {
            let _guard = self.runtime.enter();
            if let Some(rx) = event_rx {
                self.tasks
                    .push(tokio::spawn(dispatch::forward(self.events.clone(), rx)));
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
        if let Some(mut engine) = self.engine.take() {
            self.runtime
                .block_on(engine.stop())
                .map_err(|e| Error::Engine(e.to_string()))?;
            self.event_queue.push_back(Event::Stopped);
        }
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.engine.is_some()
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
