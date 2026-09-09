mod backoff;
mod cancellation;
mod liveness;
mod reconnect;
mod shutdown;
mod tls_provider;

use tokio::sync::mpsc;

use crate::upstream::{Deadlines, Upstream, UpstreamRunner, proto};

type Dialling = (
    Upstream,
    UpstreamRunner,
    mpsc::UnboundedReceiver<proto::Incoming>,
);

fn a_relay_on(port: u16) -> Dialling {
    a_relay_at(format!("ws://127.0.0.1:{port}"), Deadlines::default())
}

fn a_relay_at(url: String, deadlines: Deadlines) -> Dialling {
    let (incoming, arriving) = mpsc::unbounded_channel();
    let (handle, runner) = Upstream::pair(url, incoming, deadlines);
    (handle, runner, arriving)
}
