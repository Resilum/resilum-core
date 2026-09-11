use std::sync::Arc;

use iroh::EndpointId;
use leviculum_std::driver::ReticulumNode;
use leviculum_std::interfaces::ByteChannelHandle;

use crate::coordinates::PeerId;
use crate::discovery::{Attachments, OriginRegistry};

pub(super) const NOT_NAMED_UNTIL_THEY_ANNOUNCE: Option<PeerId> = None;

pub(super) struct Wiring {
    pub(super) engine: Arc<ReticulumNode>,
    pub(super) attachments: Arc<Attachments>,
    pub(super) origin: Arc<OriginRegistry>,
}

pub(super) fn attached_as(id: EndpointId) -> String {
    format!("iroh[{id}]")
}

pub(super) struct DetachesBothHalves {
    engine: Arc<ReticulumNode>,
    channel: Option<ByteChannelHandle>,
}

impl DetachesBothHalves {
    pub(super) fn new(engine: Arc<ReticulumNode>, channel: ByteChannelHandle) -> Self {
        Self {
            engine,
            channel: Some(channel),
        }
    }
}

impl Drop for DetachesBothHalves {
    fn drop(&mut self) {
        let Some(channel) = self.channel.take() else {
            return;
        };
        if let Err(e) = self.engine.remove_interface(channel.id()) {
            tracing::debug!(error = %e, "the node was already down when an iroh link let go");
        }
        channel.detach();
    }
}
