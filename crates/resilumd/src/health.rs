//! Liveness heartbeat. A thread refreshes a file only while the engine is
//! actually serving; the container healthcheck reads its age.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use leviculum_std::driver::ReticulumNode;
use resilum_tasks::Watching;

const INTERVAL: Duration = Duration::from_secs(10);

pub fn file_path(storage_path: Option<&Path>, env: Option<String>) -> PathBuf {
    if let Some(p) = env.filter(|p| !p.is_empty()) {
        return PathBuf::from(p);
    }
    match storage_path {
        Some(dir) => dir.join("health"),
        None => std::env::temp_dir().join("resilum-health"),
    }
}

pub fn spawn(engine: Arc<ReticulumNode>, tasks: Arc<Watching>, path: PathBuf) {
    tracing::info!(path = %path.display(), "heartbeat");
    let started = resilum_tasks::a_thread_of_its_own("the heartbeat", move || {
        loop {
            beat(&path, || alive(&engine, &tasks));
            std::thread::sleep(INTERVAL);
        }
    });
    if let Err(e) = started {
        tracing::error!(error = %e, "no thread to beat from; this node reports no health at all");
    }
}

/// A panicked event loop leaves the process up with its sockets bound, and
/// `transport_stats` blocks on the core mutex, so a deadlocked tick never
/// returns here either.
fn alive(engine: &ReticulumNode, tasks: &Watching) -> bool {
    if !engine.is_running() {
        return false;
    }
    engine.transport_stats();
    let stopped = tasks.whichever_stopped();
    if !stopped.is_empty() {
        tracing::error!(stopped = ?stopped, "these are no longer running");
        return false;
    }
    true
}

fn beat(path: &Path, alive: impl FnOnce() -> bool) -> bool {
    if !alive() {
        return false;
    }
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default();
    if let Err(e) = resilum_store::write_text(path, &format!("{stamp}\n")) {
        tracing::warn!(path = %path.display(), error = %e, "heartbeat write failed");
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().expect("a temporary directory")
    }

    #[test]
    fn a_down_engine_leaves_the_file_untouched() {
        let dir = temp_dir();
        let path = dir.path().join("health");

        assert!(!beat(&path, || false));
        assert!(!path.exists());

        resilum_store::write_text(&path, "stale\n").expect("a stale heartbeat");
        assert!(!beat(&path, || false));
        assert_eq!(
            resilum_store::read_text(&path).expect("still there"),
            "stale\n"
        );
    }

    #[test]
    fn a_live_engine_refreshes_the_file() {
        let dir = temp_dir();
        let path = dir.path().join("health");

        assert!(beat(&path, || true));
        let first = resilum_store::modified_at(&path).expect("written");

        std::thread::sleep(Duration::from_millis(1100));
        assert!(beat(&path, || true));
        let second = resilum_store::modified_at(&path).expect("written again");
        assert!(second > first);
    }

    #[tokio::test]
    async fn a_node_whose_task_died_is_not_healthy_however_well_the_engine_runs() {
        let watching = Watching::default();
        watching.keep(resilum_tasks::watch(
            "one that runs on",
            std::future::pending(),
        ));
        assert!(watching.whichever_stopped().is_empty());

        watching.keep(resilum_tasks::watch("one that ends", async {}));
        while watching.whichever_stopped().is_empty() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }

        let dir = temp_dir();
        let path = dir.path().join("health");
        assert!(!beat(&path, || watching.whichever_stopped().is_empty()));
        assert!(!path.exists());
    }

    #[test]
    fn the_env_override_wins_over_storage_path() {
        let storage = PathBuf::from("/config/state");
        assert_eq!(
            file_path(Some(&storage), Some("/run/health".into())),
            PathBuf::from("/run/health")
        );
        assert_eq!(
            file_path(Some(&storage), Some(String::new())),
            PathBuf::from("/config/state/health")
        );
        assert_eq!(
            file_path(Some(&storage), None),
            PathBuf::from("/config/state/health")
        );
    }
}
