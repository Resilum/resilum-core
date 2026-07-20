//! Shared core for a Resilum node, built on the leviculum Reticulum stack.
//! Consumed by `resilumd` and, via `resilum-ffi`, the mobile app.

mod config;
mod engine;
mod error;
mod event;
pub mod spec;
pub mod supervisor;

pub use config::Config;
pub use error::{Error, Result};
pub use event::Event;

use std::collections::VecDeque;

use leviculum_std::api::Node as LevNode;

/// A Resilum node: owns the leviculum engine (with its tokio runtime) and an
/// outbound event queue.
pub struct Node {
    config: Config,
    runtime: tokio::runtime::Runtime,
    engine: Option<LevNode>,
    events: VecDeque<Event>,
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
            events: VecDeque::new(),
        })
    }

    pub fn start(&mut self) -> Result<()> {
        if self.engine.is_some() {
            return Err(Error::AlreadyRunning);
        }
        let mut engine = engine::configure_builder(&self.config)?
            .build()
            .map_err(|e| Error::Engine(e.to_string()))?;
        // leviculum's lifecycle is async; block the caller.
        self.runtime
            .block_on(engine.start())
            .map_err(|e| Error::Engine(e.to_string()))?;
        self.engine = Some(engine);
        self.events.push_back(Event::Started);
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        if let Some(mut engine) = self.engine.take() {
            self.runtime
                .block_on(engine.stop())
                .map_err(|e| Error::Engine(e.to_string()))?;
            self.events.push_back(Event::Stopped);
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
        self.events.pop_front()
    }

    pub fn config(&self) -> &Config {
        &self.config
    }
}
