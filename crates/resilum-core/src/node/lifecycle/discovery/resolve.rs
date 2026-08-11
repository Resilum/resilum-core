//! Turning configured discovery services into the ones actually dialled.

use crate::config::DiscoveryService;

/// Whether any service asks for the in-process Tor client, which is what
/// decides if one is bootstrapped at all.
#[cfg(feature = "arti")]
pub(super) fn wants_embedded_arti(services: &[DiscoveryService]) -> bool {
    use crate::config::SocksProxy;
    services
        .iter()
        .any(|s| matches!(s.socks_proxy, Some(SocksProxy::EmbeddedArti)))
}

/// Point every `EmbeddedArti` service at the port the embedded client actually
/// bound, so the rest of the code only ever sees a concrete SOCKS address.
pub(super) fn resolve_discovery(
    services: &[DiscoveryService],
    #[cfg(feature = "arti")] arti_port: Option<u16>,
) -> Vec<DiscoveryService> {
    services
        .iter()
        .map(|s| {
            let out = s.clone();
            #[cfg(feature = "arti")]
            let out = {
                use crate::config::SocksProxy;
                let mut out = out;
                if let Some(SocksProxy::EmbeddedArti) = out.socks_proxy
                    && let Some(port) = arti_port
                {
                    out.socks_proxy = Some(SocksProxy::External("127.0.0.1".into(), port));
                }
                out
            };
            out
        })
        .collect()
}
