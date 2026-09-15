use leviculum_std::socket_hook::OutboundSocketHook;

use super::Node;
use crate::error::{Error, Result};

#[cfg(feature = "iroh")]
mod iroh;
#[cfg(unix)]
mod wifi_group;

impl Node {
    pub fn set_protect(&mut self, protect: Option<OutboundSocketHook>) {
        self.protect = protect;
    }

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
        let ours = match &self.identity {
            Some(identity) => crate::egress::own::OwnExits::of(&self.config.egress, |service| {
                crate::egress::listen::dest_hash(identity.clone(), service)
            }),
            None => crate::egress::own::OwnExits::default(),
        };
        let params = crate::egress::vpn::VpnParams {
            engine,
            router,
            registry: self.registry.clone(),
            policy,
            own: ours,
            mtu,
            ygg_fd,
            flows: self.nursery.clone(),
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

    #[cfg(all(unix, feature = "ygg"))]
    pub fn ygg_attach(
        &self,
        ygg_fd: std::os::fd::RawFd,
        the_address_the_engine_answers_on: &str,
    ) -> Result<crate::ygg::YggHandle> {
        let ygg_address = the_address_the_engine_answers_on;
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
            crate::discovery::store::say_the_address_is(path, ygg_address)
                .map_err(|e| Error::Ygg(format!("write ygg address to {}: {e}", path.display())))?;
        }
        let rns_port = service.rns_port;
        let socks_port = a_loopback_proxy_to_dial_peers_through(service);
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
        let handle = crate::ygg::attach(crate::ygg::Attaching {
            engine,
            origin: self.origin_registry.clone(),
            ygg_fd,
            ygg_address,
            rns_port,
            socks_port,
            on_detach,
            conns: self.nursery.clone(),
        })
        .map_err(|e| Error::Ygg(e.to_string()))?;
        discovery.activate();
        self.discovery_trigger.notify_waiters();
        Ok(handle)
    }
}

#[cfg(all(unix, feature = "ygg"))]
fn a_loopback_proxy_to_dial_peers_through(
    service: &crate::config::DiscoveryService,
) -> Option<u16> {
    match &service.socks_proxy {
        Some(crate::config::SocksProxy::External(_, port)) => Some(*port),
        _ => None,
    }
}
