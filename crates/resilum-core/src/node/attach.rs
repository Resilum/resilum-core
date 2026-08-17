//! Data-plane attach points: the socket-protect hook, the L3 VPN routing hub,
//! and the Yggdrasil transport.

use std::collections::{HashMap, HashSet};

use leviculum_std::socket_hook::OutboundSocketHook;

use super::Node;
use crate::error::{Error, Result};

impl Node {
    /// Register a hook run on every outbound socket before it connects, so an
    /// embedder can keep upstream sockets off a captured tun (via the host's
    /// socket-protection API). Takes effect on the next `start`.
    pub fn set_protect(&mut self, protect: Option<OutboundSocketHook>) {
        self.protect = protect;
    }

    /// Attach an L3 routing hub to `tun_fd`, forwarding its TCP flows through
    /// the egress mesh. Requires a running node with an ingress policy.
    #[cfg(unix)]
    pub fn vpn_attach(
        &self,
        tun_fd: std::os::fd::RawFd,
        mtu: usize,
        ygg_fd: Option<std::os::fd::RawFd>,
    ) -> Result<crate::egress::vpn::VpnHandle> {
        let engine = self.engine.clone().ok_or(Error::NotRunning)?;
        let router = self.router.clone().ok_or(Error::NotRunning)?;
        let policy = self.config.ingress.clone().ok_or(Error::VpnNoIngress)?;
        let mut skip: HashMap<String, HashSet<Vec<u8>>> = HashMap::new();
        if let Some(identity) = &self.identity {
            for own in &self.config.egress {
                let hash = crate::egress::listen::dest_hash(identity.clone(), &own.service);
                skip.entry(own.service.clone()).or_default().insert(hash);
            }
        }
        let params = crate::egress::vpn::VpnParams {
            engine,
            router,
            registry: self.registry.clone(),
            policy,
            skip,
            mtu,
            ygg_fd,
            #[cfg(feature = "arti")]
            tor: self
                .embedded_tor
                .as_ref()
                .map(crate::tor::EmbeddedTor::client),
            #[cfg(feature = "i2p")]
            i2p: Some(std::sync::Arc::new(
                crate::egress::vpn::I2pConduit::default(),
            )),
        };
        let _guard = self.runtime.enter();
        crate::egress::vpn::attach(params, tun_fd).map_err(|e| Error::Vpn(e.to_string()))
    }

    /// Attach the Yggdrasil packet conduit `ygg_fd`, accepting RNS links over
    /// ygg. `ygg_address` is the engine's own `200::/7` address (the caller reads
    /// it from `GetAddressString`); it is written to the yggdrasil discovery
    /// service's `hostname_path` so the node announces where peers should dial —
    /// the engine runs `IfName=none`, so there is no OS ygg interface to
    /// auto-detect it from. The RNS port comes from that service. Requires a
    /// running node with yggdrasil discovery configured.
    #[cfg(all(unix, feature = "ygg"))]
    pub fn ygg_attach(
        &self,
        ygg_fd: std::os::fd::RawFd,
        ygg_address: &str,
    ) -> Result<crate::ygg::YggHandle> {
        let service = self
            .config
            .discovery
            .iter()
            .find(|s| {
                matches!(
                    s.endpoint_format,
                    crate::config::EndpointFormat::BracketedIpv6
                )
            })
            .ok_or_else(|| Error::Ygg("no yggdrasil discovery service configured".into()))?;
        if let Some(path) = &service.hostname_path {
            std::fs::write(path, ygg_address)
                .map_err(|e| Error::Ygg(format!("write ygg address to {}: {e}", path.display())))?;
        }
        let rns_port = service.rns_port;
        // A loopback SOCKS proxy the discovery service dials ygg peers through
        // (initiate side); absent when the service dials `200::/7` directly
        // (an ygg tun is present).
        let socks_port = match &service.socks_proxy {
            Some(crate::config::SocksProxy::External(_, port)) => Some(*port),
            _ => None,
        };
        let engine = self.engine.clone().ok_or(Error::NotRunning)?;
        let discovery = self
            .ygg_discovery
            .clone()
            .ok_or_else(|| Error::Ygg("no yggdrasil discovery service configured".into()))?;
        let on_detach = {
            let discovery = discovery.clone();
            Box::new(move || discovery.deactivate())
        };
        let _guard = self.runtime.enter();
        let handle = crate::ygg::attach(
            engine,
            self.origin_registry.clone(),
            ygg_fd,
            ygg_address,
            rns_port,
            socks_port,
            on_detach,
        )
        .map_err(|e| Error::Ygg(e.to_string()))?;
        // After attach, so warm_start's dials reach the SOCKS proxy it just spawned.
        discovery.activate();
        self.discovery_trigger.notify_waiters();
        Ok(handle)
    }

    /// Attach the in-process iroh transport: bind the endpoint, accept inbound
    /// links, and dial the configured bootstrap peers. Requires a running node
    /// with an `[iroh]` config.
    #[cfg(feature = "iroh")]
    pub fn iroh_attach(&self) -> Result<crate::iroh::IrohHandle> {
        let engine = self.engine.clone().ok_or(Error::NotRunning)?;
        let cfg = self
            .config
            .iroh
            .clone()
            .ok_or_else(|| Error::Iroh("iroh not configured".into()))?;
        let dir = self.config.storage_path.clone().unwrap_or_else(|| {
            std::env::temp_dir().join(format!("resilum-{}", self.config.instance_name))
        });
        let discovery = self.iroh_discovery.clone();
        let protect = self.protect.clone();
        let handle = self
            .runtime
            .block_on(crate::iroh::attach(engine, &dir, &cfg, discovery, protect))
            .map_err(Error::Iroh)?;
        self.discovery_trigger.notify_waiters();
        Ok(handle)
    }
}
