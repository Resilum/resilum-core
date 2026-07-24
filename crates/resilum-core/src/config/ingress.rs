#[derive(Clone, Debug)]
pub struct IngressConfig {
    pub services: Vec<String>,
    pub listen_tcp: String,
    /// `smart` | `true` | `false`.
    pub use_own: String,
    pub allow_country: Vec<String>,
    pub deny_country: Vec<String>,
    /// Explicit destination hash to dial, bypassing discovery.
    pub target: Option<[u8; 16]>,
    /// Custom probe targets (IPv4 literal `host:port`), override the env var
    /// and built-in defaults when non-empty.
    pub probe_targets: Vec<(String, u16)>,
}

impl IngressConfig {
    pub fn new(service: impl Into<String>, listen_tcp: impl Into<String>) -> Self {
        Self {
            services: vec![service.into()],
            listen_tcp: listen_tcp.into(),
            use_own: "smart".into(),
            allow_country: Vec::new(),
            deny_country: Vec::new(),
            target: None,
            probe_targets: Vec::new(),
        }
    }
}
