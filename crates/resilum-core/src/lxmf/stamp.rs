//! Proof-of-work for peers that price one, mined off the core lock.

mod jobs;

use std::sync::mpsc::Sender;
use std::thread;

use leviculum_lxmf::{CooperativeStamper, DeliveryStampRequest, PropagationStampRequest};
use tokio::sync::mpsc::UnboundedReceiver;

pub(super) use jobs::Jobs;

use super::handle::Command;

/// The two kinds are the same proof of work — `StampExecutor::generate` — over
/// different bytes: the recipient stamp over the message id, the propagation
/// node's over the transient id of the prepared envelope, each with its own
/// workblock expansion. So one executor answers both.
pub(super) enum Job {
    Delivery(DeliveryStampRequest),
    Propagation(PropagationStampRequest),
}

pub(super) enum Outcome {
    Delivery {
        request: DeliveryStampRequest,
        stamp: Result<[u8; 32], String>,
    },
    Propagation {
        request: PropagationStampRequest,
        stamp: Result<[u8; 32], String>,
    },
}

/// Its own thread rather than `tokio::spawn`: the grind is unbounded pure CPU
/// and would starve a runtime worker for its duration.
///
/// One thread for both kinds. A second would double what the process spends on
/// stamps to buy parallelism the router cannot use: it wants one stamp at a
/// time per message, and both queues are drained by the same 4-second retry.
pub(super) fn spawn(mut jobs: UnboundedReceiver<Job>, out: Sender<Command>) {
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
            while let Some(job) = jobs.recv().await {
                if out.send(Command::Stamp(grind(job).await)).is_err() {
                    return;
                }
            }
        });
    });
}

async fn grind(job: Job) -> Outcome {
    let mut stamper = CooperativeStamper::cooperative(rand_core::OsRng);
    match job {
        Job::Delivery(request) => Outcome::Delivery {
            stamp: request
                .generate_with(&mut stamper)
                .await
                .map_err(|e| format!("{e:?}")),
            request,
        },
        Job::Propagation(request) => Outcome::Propagation {
            stamp: request
                .generate_with(&mut stamper)
                .await
                .map_err(|e| format!("{e:?}")),
            request,
        },
    }
}
