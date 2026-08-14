//! Which nameservers iroh's resolver was handed, logged once at endpoint build.
//!
//! Without this the only way to tell a system nameserver from the public
//! fallback is to read the kernel's socket tables, which no field report can do.

use std::sync::Once;

use tracing::{info, warn};

pub(super) fn log_system() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| match read() {
        Ok(nameservers) if nameservers.is_empty() => {
            warn!("dns: system configuration lists no nameserver, falling back to public dns");
        }
        Ok(nameservers) => info!(nameservers, "dns: from system configuration"),
        Err(reason) => warn!(
            reason,
            "dns: no system configuration, falling back to public dns"
        ),
    });
}

/// Reads the same source iroh's resolver reads: `resolv.conf` on unix, and on
/// Android the active network's `LinkProperties`, which needs the JavaVM and
/// Context installed first — an uninstalled one panics rather than erroring.
fn read() -> Result<String, String> {
    #[cfg(target_os = "android")]
    let conf = std::panic::catch_unwind(hickory_resolver::system_conf::read_system_conf)
        .map_err(|_| "android platform context is not installed".to_owned())?;
    #[cfg(not(target_os = "android"))]
    let conf = hickory_resolver::system_conf::read_system_conf();

    let (config, _opts) = conf.map_err(|e| e.to_string())?;
    Ok(config
        .name_servers()
        .iter()
        .map(|n| n.ip.to_string())
        .collect::<Vec<_>>()
        .join(", "))
}
