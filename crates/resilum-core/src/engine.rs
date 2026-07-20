//! Builds leviculum interfaces from `Config`: a TCP listener, TCP clients to
//! anchors, and the local-segment AutoInterface.

use leviculum_std::api::{self, NodeBuilder};

use crate::{Config, Error, Result};

pub(crate) fn configure_builder(config: &Config) -> Result<NodeBuilder> {
    // TODO: load-or-generate a stable identity from `storage_path`.
    let mut builder = NodeBuilder::new().identity(api::generate_identity());

    if let Some(path) = &config.storage_path {
        builder = builder.storage_path(path.clone());
    }
    if let Some(listen) = &config.listen {
        let addr = listen
            .parse()
            .map_err(|_| Error::Config(format!("bad listen address: {listen}")))?;
        builder = builder.add_tcp_server(addr);
    }
    for anchor in &config.bootstrap {
        let addr = anchor
            .parse()
            .map_err(|_| Error::Config(format!("bad bootstrap address: {anchor}")))?;
        builder = builder.add_tcp_client(addr);
    }
    if config.discover_interfaces {
        builder = builder.add_auto_interface();
    }
    Ok(builder)
}

#[cfg(test)]
mod tests {
    use super::configure_builder;
    use crate::{Config, Error};

    #[test]
    fn maps_a_wellformed_config() {
        let cfg = Config {
            listen: Some("[::]:4242".into()),
            bootstrap: vec!["203.0.113.10:4242".into()],
            ..Config::minimal("test")
        };
        assert!(configure_builder(&cfg).is_ok());
    }

    #[test]
    fn rejects_bad_listen() {
        let cfg = Config {
            listen: Some("nope".into()),
            ..Config::minimal("test")
        };
        assert!(matches!(configure_builder(&cfg), Err(Error::Config(_))));
    }

    #[test]
    fn rejects_bad_bootstrap() {
        let cfg = Config {
            bootstrap: vec!["nope".into()],
            ..Config::minimal("test")
        };
        assert!(matches!(configure_builder(&cfg), Err(Error::Config(_))));
    }
}
