#[derive(Clone, Debug)]
pub struct IngressConfig {
    pub services: Vec<String>,
    pub listen_tcp: Option<String>,
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
        Self::without_a_listener(service).listening_on(listen_tcp)
    }

    pub fn without_a_listener(service: impl Into<String>) -> Self {
        Self {
            services: vec![service.into()],
            listen_tcp: None,
            use_own: "smart".into(),
            allow_country: Vec::new(),
            deny_country: Vec::new(),
            target: None,
            probe_targets: Vec::new(),
        }
    }

    #[must_use]
    pub fn listening_on(mut self, listen_tcp: impl Into<String>) -> Self {
        self.listen_tcp = Some(listen_tcp.into());
        self
    }
}
