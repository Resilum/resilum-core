mod accessors;
mod attach;
mod coordinates;
mod directory;
mod interface;
mod lifecycle;

use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::AtomicU16;
use std::sync::{Arc, Mutex};

use leviculum_std::api::Identity;
use leviculum_std::driver::ReticulumNode;
use leviculum_std::socket_hook::OutboundSocketHook;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

use crate::config::Config;
use crate::discovery::Service;
use crate::dispatch;
use crate::egress::CandidateRegistry;
use crate::error::{Error, Result};
use crate::event;
use crate::link::LinkRouter;
use crate::mirrors;

/// A Resilum node: owns the leviculum engine (with its tokio runtime) and an
/// outbound event queue.
pub struct Node {
    pub(crate) config: Config,
    pub(crate) runtime: tokio::runtime::Runtime,
    pub(crate) engine: Option<Arc<ReticulumNode>>,
    pub(crate) registry: Arc<CandidateRegistry>,
    pub(crate) router: Option<Arc<LinkRouter>>,
    pub(crate) identity: Option<Identity>,
    pub(crate) lxmf: Option<Arc<crate::lxmf::LxmfHandle>>,
    pub(crate) protect: Option<OutboundSocketHook>,
    pub(crate) events: dispatch::Events,
    pub(crate) tasks: Vec<JoinHandle<()>>,
    pub(crate) event_queue: event::Queue,
    pub(crate) socks_port: Arc<AtomicU16>,
    pub(crate) discovery_trigger: Arc<Notify>,
    pub(crate) mirror_registry: Option<Arc<mirrors::Registry>>,
    pub(crate) origin_registry: Arc<crate::discovery::OriginRegistry>,
    pub(crate) directories: BTreeMap<Service, Arc<crate::discovery::ServiceDirectory>>,
    pub(crate) coordinates: Arc<crate::coordinates::Coordinates>,
    pub(crate) attachments: Arc<crate::discovery::Attachments>,
    pub(crate) covert_listeners: Vec<leviculum_std::interfaces::ByteChannelHandle>,
    #[cfg(all(unix, feature = "ygg"))]
    pub(crate) ygg_discovery: Option<Arc<crate::discovery::TcpDiscovered>>,
    #[cfg(feature = "iroh")]
    pub(crate) iroh_discovery: Option<Arc<crate::iroh::IrohDiscovery>>,
    #[cfg(feature = "arti")]
    pub(crate) embedded_tor: Option<crate::tor::EmbeddedTor>,
}

impl Node {
    pub fn new(config: Config) -> Result<Self> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|e| Error::Engine(format!("tokio runtime: {e}")))?;
        let coordinates: Arc<crate::coordinates::Coordinates> = Arc::default();
        Ok(Self {
            config,
            runtime,
            engine: None,
            registry: Arc::new(CandidateRegistry::default()),
            router: None,
            identity: None,
            lxmf: None,
            protect: None,
            events: dispatch::Events::new(1024),
            tasks: Vec::new(),
            event_queue: Arc::new(Mutex::new(VecDeque::new())),
            socks_port: Arc::new(AtomicU16::new(0)),
            discovery_trigger: Arc::new(Notify::new()),
            mirror_registry: None,
            origin_registry: Arc::new(crate::discovery::OriginRegistry::default()),
            directories: Service::at_mesh_addresses()
                .map(|service| (service, Arc::default()))
                .collect(),
            attachments: Arc::new(crate::discovery::Attachments::new(Arc::clone(&coordinates))),
            covert_listeners: Vec::new(),
            coordinates,
            #[cfg(all(unix, feature = "ygg"))]
            ygg_discovery: None,
            #[cfg(feature = "iroh")]
            iroh_discovery: None,
            #[cfg(feature = "arti")]
            embedded_tor: None,
        })
    }
}
