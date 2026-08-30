pub const UDP_LISTEN_UNLESS_TOLD_OTHERWISE: &str = "0.0.0.0:4242";

#[derive(Clone, Debug, Default)]
pub struct UdpInterface {
    pub listen: Option<String>,
    pub peers_every_datagram_goes_to: Vec<String>,
}

impl UdpInterface {
    #[must_use]
    pub fn bound_to(&self) -> &str {
        self.listen
            .as_deref()
            .unwrap_or(UDP_LISTEN_UNLESS_TOLD_OTHERWISE)
    }

    #[must_use]
    pub fn carries_nothing(&self) -> bool {
        self.peers_every_datagram_goes_to.is_empty()
    }
}
