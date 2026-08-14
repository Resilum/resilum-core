//! Builds the in-process iroh endpoint from `IrohConfig`.

use iroh::address_lookup::{DnsAddressLookup, PkarrPublisher, PkarrResolver};
use iroh::endpoint::Builder;
use iroh::endpoint::presets::Minimal;
use iroh::{Endpoint, RelayMap, RelayMode, SecretKey};
use leviculum_std::socket_hook::OutboundSocketHook;

use crate::config::IrohConfig;

pub(crate) const ALPN: &[u8] = b"resilum/iroh/1";

/// Bind an endpoint that accepts our ALPN. Resolves peers by `EndpointId`
/// (`PkarrResolver` + DNS) but only publishes its own address when `publish` is
/// set, so a leaf never beacons where it is. With `iroh-protect` and a socket
/// hook present, the built-in UDP transport is swapped for a protected one
/// (mobile), keeping relay so it stays a full participant — see
/// [`super::transport`].
pub async fn build(
    secret: SecretKey,
    cfg: &IrohConfig,
    protect: Option<OutboundSocketHook>,
) -> Result<Endpoint, String> {
    #[cfg(feature = "iroh-protect")]
    if let Some(hook) = protect {
        return super::transport::build_protected(secret, cfg, hook).await;
    }
    #[cfg(not(feature = "iroh-protect"))]
    let _ = protect;
    base_builder(secret, cfg)?
        .bind()
        .await
        .map_err(|e| e.to_string())
}

/// Common configuration shared by the stable and protected builders: ALPN,
/// relay, resolver, and the optional publisher.
pub(super) fn base_builder(secret: SecretKey, cfg: &IrohConfig) -> Result<Builder, String> {
    super::resolvers::log_system();
    let mut builder = Endpoint::builder(Minimal)
        .secret_key(secret)
        .alpns(vec![ALPN.to_vec()])
        .relay_mode(relay_mode(cfg)?)
        .address_lookup(PkarrResolver::n0_dns())
        .address_lookup(DnsAddressLookup::n0_dns());
    if cfg.publish {
        builder = builder.address_lookup(PkarrPublisher::n0_dns());
    }
    Ok(builder)
}

fn relay_mode(cfg: &IrohConfig) -> Result<RelayMode, String> {
    match &cfg.relay {
        None => Ok(RelayMode::Default),
        Some(url) => RelayMap::try_from_iter([url.as_str()])
            .map(RelayMode::Custom)
            .map_err(|e| format!("invalid iroh relay url {url:?}: {e}")),
    }
}
