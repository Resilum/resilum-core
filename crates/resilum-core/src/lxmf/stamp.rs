//! Proof-of-work for peers that price one, mined off the core lock.

use std::sync::mpsc::Sender;
use std::thread;

use leviculum_lxmf::{CooperativeStamper, DeliveryStampRequest};
use tokio::sync::mpsc::UnboundedReceiver;

use super::handle::Command;

pub(super) enum Outcome {
    Ready {
        request: DeliveryStampRequest,
        stamp: [u8; 32],
    },
    Failed {
        request: DeliveryStampRequest,
        detail: String,
    },
}

/// Its own thread rather than `tokio::spawn`, which the mine now accepts: the
/// grind is unbounded pure CPU and would starve a worker for its duration.
pub(super) fn spawn(mut requests: UnboundedReceiver<DeliveryStampRequest>, out: Sender<Command>) {
    thread::spawn(move || {
        let runtime = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(e) => {
                tracing::error!(error = %e, "no runtime for the lxmf stamp executor");
                return;
            }
        };
        runtime.block_on(async move {
            while let Some(request) = requests.recv().await {
                let mut stamper = CooperativeStamper::cooperative(rand_core::OsRng);
                let outcome = match request.generate_with(&mut stamper).await {
                    Ok(stamp) => Outcome::Ready { request, stamp },
                    Err(e) => Outcome::Failed {
                        request,
                        detail: format!("{e:?}"),
                    },
                };
                if out.send(Command::Stamp(outcome)).is_err() {
                    return;
                }
            }
        });
    });
}
