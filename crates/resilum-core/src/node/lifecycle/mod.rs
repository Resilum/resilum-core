mod coordinates;
mod discovery;
mod egress;
mod mirrors;

use std::sync::Arc;

use tokio::sync::mpsc;

use super::Node;
use crate::error::{Error, Result};
use crate::event::{self, Event};
use crate::{bridge, dispatch, engine, link, lxmf, supervisor};

impl Node {
    pub fn start(&mut self) -> Result<()> {
        if self.engine.is_some() {
            return Err(Error::AlreadyRunning);
        }
        let (builder, identity) = engine::build_node(&self.config, self.protect.clone())?;
        // Before `build_sync`, because that is where a core processor is
        // installed — see `lxmf::install`.
        let builder = match &self.config.lxmf {
            Some(cfg) => {
                let storage = self.config.storage_path.clone();
                let (builder, handle) = lxmf::install(builder, cfg, &identity, storage.as_deref());
                self.lxmf = Some(Arc::new(handle));
                builder
            }
            None => builder,
        };
        let mut leviculum = builder
            .build_sync()
            .map_err(|e| Error::Engine(e.to_string()))?;
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

            let inbox = Arc::new(link::Inbox::default());
            let (unclaimed_tx, unclaimed_rx) = mpsc::unbounded_channel();
            discovery::bring_up(self, &leviculum, &identity)?;
            coordinates::bring_up(
                self,
                &leviculum,
                &identity,
                &router,
                &inbox,
                inbound_rx,
                unclaimed_tx,
            );
            egress::bring_up(self, &leviculum, &identity, &router, unclaimed_rx);
            mirrors::bring_up(self, &leviculum, &identity);

            if let Some(rx) = event_rx {
                self.tasks
                    .push(tokio::spawn(dispatch::forward(self.events.clone(), rx)));
            }
            self.tasks.extend(supervisor::spawn_all(bridge_tasks));
            self.router = Some(router.clone());
        }
        self.identity = Some(identity);
        self.engine = Some(leviculum);
        event::push(&self.event_queue, Event::Started);
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        self.wait_for_tasks_to_let_go_of_the_engine();
        self.router = None;
        self.identity = None;
        self.lxmf = None;
        #[cfg(all(unix, feature = "ygg"))]
        {
            self.ygg_discovery = None;
        }
        #[cfg(feature = "iroh")]
        {
            self.iroh_discovery = None;
        }
        #[cfg(feature = "arti")]
        {
            self.embedded_tor = None;
        }
        self.directories.clear();
        if let Some(leviculum) = self.engine.take() {
            let mut leviculum = Arc::try_unwrap(leviculum).map_err(|still_shared| {
                Error::Engine(format!(
                    "{} holders of the engine outlived stop; its ports stay bound",
                    Arc::strong_count(&still_shared)
                ))
            })?;
            self.runtime
                .block_on(leviculum.stop())
                .map_err(|e| Error::Engine(e.to_string()))?;
            event::push(&self.event_queue, Event::Stopped);
        }
        Ok(())
    }

    fn wait_for_tasks_to_let_go_of_the_engine(&mut self) {
        let tasks: Vec<_> = self.tasks.drain(..).collect();
        for task in &tasks {
            task.abort();
        }
        self.runtime.block_on(async {
            for task in tasks {
                let _ = task.await;
            }
        });
    }
}
