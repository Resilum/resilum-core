use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tokio::sync::mpsc;

use super::Node;
use crate::error::{Error, Result};
use crate::event::{self, Event};
use crate::{bridge, discovery, dispatch, egress, engine, link, supervisor};

impl Node {
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
            if !self.config.discovery.is_empty() {
                let discovery = Arc::new(discovery::build_from_services(
                    &self.config.discovery,
                    engine.clone(),
                    self.discovery_trigger.clone(),
                ));
                let bus = self.events.subscribe();
                self.tasks
                    .push(tokio::spawn(discovery::run_consume(discovery.clone(), bus)));
                let destinations = discovery::build_destinations(
                    &engine,
                    identity.clone(),
                    &self.config.discovery,
                )?;
                self.tasks.push(tokio::spawn(discovery::run_produce(
                    engine.clone(),
                    discovery,
                    destinations,
                    self.config.discovery_announce_interval,
                    self.discovery_trigger.clone(),
                )));
            }
            if let Some(connect) = self.config.connect.clone() {
                // Skip this node's own egress announces when selecting a peer.
                let mut skip: HashMap<String, HashSet<Vec<u8>>> = HashMap::new();
                for own in &self.config.egress {
                    let hash = egress::listen::dest_hash(identity.clone(), &own.service);
                    skip.entry(own.service.clone()).or_default().insert(hash);
                }
                let active = Arc::new(egress::ActiveLinks::default());
                for service in &connect.services {
                    let bus = self.events.subscribe();
                    self.tasks.push(tokio::spawn(egress::discover::run(
                        engine.clone(),
                        self.registry.clone(),
                        active.clone(),
                        self.event_queue.clone(),
                        service.clone(),
                        bus,
                    )));
                }
                self.tasks.push(tokio::spawn(egress::connect::run(
                    engine.clone(),
                    router.clone(),
                    self.registry.clone(),
                    active,
                    self.socks_port.clone(),
                    connect.clone(),
                    skip.clone(),
                )));
                self.tasks.push(tokio::spawn(egress::monitor::run(
                    engine.clone(),
                    router.clone(),
                    self.registry.clone(),
                    connect,
                    skip,
                )));
            }
            if let Some(rx) = event_rx {
                self.tasks
                    .push(tokio::spawn(dispatch::forward(self.events.clone(), rx)));
            }
            if !self.config.egress.is_empty() {
                self.tasks.push(tokio::spawn(egress::listen::run(
                    engine.clone(),
                    identity,
                    self.config.egress.clone(),
                    inbound_rx,
                )));
            }
            self.tasks.extend(supervisor::spawn_all(bridge_tasks));
        }
        self.engine = Some(engine);
        event::push(&self.event_queue, Event::Started);
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
            event::push(&self.event_queue, Event::Stopped);
        }
        Ok(())
    }
}
