mod discovery;
mod egress;

use std::sync::Arc;

use tokio::sync::mpsc;

use super::Node;
use crate::error::{Error, Result};
use crate::event::{self, Event};
use crate::{bridge, dispatch, engine, link, supervisor};

impl Node {
    pub fn start(&mut self) -> Result<()> {
        if self.engine.is_some() {
            return Err(Error::AlreadyRunning);
        }
        let (builder, identity) = engine::build_node(&self.config)?;
        let mut leviculum = builder.build().map_err(|e| Error::Engine(e.to_string()))?;
        self.runtime
            .block_on(leviculum.start())
            .map_err(|e| Error::Engine(e.to_string()))?;
        let event_rx = leviculum.take_event_receiver();
        let leviculum = Arc::new(leviculum);
        let bridge_tasks = bridge::tasks_for(&self.config.specs);
        {
            let _guard = self.runtime.enter();
            let router = Arc::new(link::LinkRouter::default());
            let (inbound_tx, inbound_rx) = mpsc::unbounded_channel();
            let link_bus = self.events.subscribe();
            self.tasks.push(tokio::spawn(link::run(
                router.clone(),
                link_bus,
                inbound_tx,
            )));

            discovery::bring_up(self, &leviculum, &identity)?;
            egress::bring_up(self, &leviculum, &identity, &router, inbound_rx);

            if let Some(rx) = event_rx {
                self.tasks
                    .push(tokio::spawn(dispatch::forward(self.events.clone(), rx)));
            }
            self.tasks.extend(supervisor::spawn_all(bridge_tasks));
        }
        self.engine = Some(leviculum);
        event::push(&self.event_queue, Event::Started);
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        for task in self.tasks.drain(..) {
            task.abort();
        }
        if let Some(leviculum) = self.engine.take() {
            match Arc::try_unwrap(leviculum) {
                Ok(mut leviculum) => self
                    .runtime
                    .block_on(leviculum.stop())
                    .map_err(|e| Error::Engine(e.to_string()))?,
                Err(_shared) => {}
            }
            event::push(&self.event_queue, Event::Stopped);
        }
        #[cfg(feature = "arti")]
        {
            self.embedded_tor = None;
        }
        Ok(())
    }
}
