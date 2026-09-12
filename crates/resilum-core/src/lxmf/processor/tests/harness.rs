//! A processor driven against a real core, router and propagation node.
//!
//! Nothing here stands in for the router: the events under test are the ones
//! `LxmfRouter` emits for a propagated message whose selected node prices the
//! deposit.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;

use leviculum_core::DestinationHash;
use leviculum_core::node::NodeCoreBuilder;
use leviculum_core::transport::TickOutput;
use leviculum_lxmf::PropagationStampRequest;
use leviculum_lxmf::router::{RouterEvent, RouterOutput};
use leviculum_std::api::Identity;
use leviculum_std::driver::{CoreProcessor, StdClock, StdNodeCore, StdStorage};
use tokio::sync::mpsc::UnboundedReceiver;

use super::super::{LxmfProcessor, Ready, Wiring};
use super::node::{copy_of, select_propagation_node};
use crate::config::LxmfConfig;
use crate::lxmf::handle::{Command, LxmfHandle, channel};
use crate::lxmf::inbox::Inbox;
use crate::lxmf::stamp::{Job, Jobs};

/// Every propagation node the harness announces prices its deposit at this,
/// which is non-zero so the router really asks for a stamp and small enough to
/// grind inside a test.
pub(super) const STAMP_COST: u64 = 4;

pub(super) struct Harness {
    pub(super) core: StdNodeCore,
    pub(super) processor: LxmfProcessor,
    pub(super) ready: Box<Ready>,
    pub(super) jobs: UnboundedReceiver<Job>,
    pub(super) commands: Sender<Command>,
    pub(super) handle: LxmfHandle,
    pub(super) identity: Identity,
    pub(super) propagation_node: DestinationHash,
    _storage_goes_with_the_harness: tempfile::TempDir,
}

impl Harness {
    /// A registered processor whose router has one announced propagation node
    /// selected, asking `stamp_cost` bits for a deposit.
    pub(super) fn new(tag: &str, stamp_cost: u64) -> Self {
        let dir = tempfile::Builder::new()
            .prefix(&format!("resilum-stamp-{tag}-"))
            .tempdir()
            .expect("a temporary directory");
        let identity = crate::identity::generate();
        let storage = StdStorage::new(dir.path()).expect("storage");
        let mut core = NodeCoreBuilder::new().identity(copy_of(&identity)).build(
            rand_core::OsRng,
            StdClock::new(),
            storage,
        );

        let inbox = Arc::new(Inbox::ephemeral());
        let registered = Arc::new(AtomicBool::new(false));
        let (handle, command_rx, events) = channel(
            crate::identity::lxmf_address_hex(&identity),
            registered.clone(),
            inbox.clone(),
        );
        let (job_tx, jobs) = tokio::sync::mpsc::unbounded_channel();
        let mut processor = LxmfProcessor::new(
            LxmfConfig::default(),
            copy_of(&identity),
            Wiring {
                commands: command_rx,
                events,
                inbox,
                stamps: Jobs::new(job_tx),
                registered,
                router_state: handle.router_state(),
                checkpoint: None,
            },
        );
        // Registers the delivery destination; its packets go nowhere, as this
        // node has no interfaces.
        let _registration = processor.on_tick(&mut core, 0);
        let mut ready = processor.take_ready(&mut core).expect("registered");
        let propagation_node = select_propagation_node(&mut ready, &mut core, stamp_cost);
        Self {
            commands: handle.sender(),
            core,
            processor,
            ready,
            jobs,
            handle,
            identity,
            propagation_node,
            _storage_goes_with_the_harness: dir,
        }
    }

    /// One router pass, drained the way the tick hook drains it.
    pub(super) fn tick(&mut self) {
        let mut out = TickOutput::empty();
        let output = self.ready.router.tick(&mut self.core).expect("router tick");
        self.processor
            .absorb(&mut self.ready, &mut self.core, output, &mut out);
    }

    /// One command-queue pass, the way both hooks run it.
    pub(super) fn pump(&mut self) {
        let mut out = TickOutput::empty();
        self.processor
            .pump_commands(&mut self.ready, &mut self.core, &mut out);
    }

    /// Publishes the router's current state to the handle the way `park` does
    /// after every real hook, without disturbing the router the test drives
    /// directly through `ready`.
    pub(super) fn publish(&self) {
        self.processor.publish(&self.ready);
    }

    /// The same ask again, which is what the router does every processing
    /// interval until the stamp lands.
    pub(super) fn ask_again(&mut self, request: PropagationStampRequest) {
        let output = RouterOutput {
            core: TickOutput::empty(),
            events: vec![RouterEvent::PropagationStampPending(request)],
        };
        let mut out = TickOutput::empty();
        self.processor
            .absorb(&mut self.ready, &mut self.core, output, &mut out);
    }
}
