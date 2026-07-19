//! resilum-core — shared core for a Resilum node.
//!
//! Single source of truth consumed by the `resilumd` daemon and by the mobile
//! app (through `resilum-ffi`). Transport, discovery and egress-policy logic
//! will live here on top of a Rust Reticulum stack (to be selected — see the
//! repo README's "gate"). This is a skeleton: the API surface is defined, the
//! implementation is stubbed.

mod config;
mod error;
mod event;

pub use config::Config;
pub use error::{Error, Result};
pub use event::Event;

use std::collections::VecDeque;

/// A Resilum node. Owns the transport stack and an outbound event queue.
pub struct Node {
    config: Config,
    running: bool,
    events: VecDeque<Event>,
}

impl Node {
    /// Build a node from config. Does not start any I/O.
    pub fn new(config: Config) -> Result<Self> {
        Ok(Self {
            config,
            running: false,
            events: VecDeque::new(),
        })
    }

    /// Start the node: bring up enabled transports and the discovery loop.
    pub fn start(&mut self) -> Result<()> {
        if self.running {
            return Err(Error::AlreadyRunning);
        }
        // TODO: initialise the Reticulum stack and enabled interfaces from
        // `self.config`, then start discovery + egress policy.
        self.running = true;
        self.events.push_back(Event::Started);
        Ok(())
    }

    /// Stop the node and release transports.
    pub fn stop(&mut self) -> Result<()> {
        if !self.running {
            return Ok(());
        }
        // TODO: tear down interfaces and background tasks.
        self.running = false;
        self.events.push_back(Event::Stopped);
        Ok(())
    }

    /// Whether the node is running.
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Send `data` to a destination hash.
    pub fn send(&mut self, _dest: &[u8], _data: &[u8]) -> Result<()> {
        if !self.running {
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
    fn lifecycle_state_machine() {
        let mut node = Node::new(Config::minimal("test")).unwrap();
        assert!(!node.is_running());

        node.start().unwrap();
        assert!(node.is_running());
        assert_eq!(node.poll_event(), Some(Event::Started));

        // sending requires a running node
        assert!(node.send(b"dest", b"payload").is_ok());

        // double start is rejected
        assert!(matches!(node.start(), Err(Error::AlreadyRunning)));

        node.stop().unwrap();
        assert!(!node.is_running());
        assert_eq!(node.poll_event(), Some(Event::Stopped));

        // sending on a stopped node fails
        assert!(matches!(node.send(b"dest", b"payload"), Err(Error::NotRunning)));
    }
}
