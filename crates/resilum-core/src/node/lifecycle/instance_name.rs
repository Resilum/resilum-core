use std::time::{Duration, Instant};

const LONGER_THAN_A_BACKGROUND_RUNTIME_TAKES_TO_BURN_DOWN: Duration = Duration::from_secs(1);
const BETWEEN_ASKS: Duration = Duration::from_millis(5);

pub(super) fn wait_until_free(instance_name: &str) {
    let someone_else_holds_it_by_now =
        Instant::now() + LONGER_THAN_A_BACKGROUND_RUNTIME_TAKES_TO_BURN_DOWN;
    while taken(instance_name) {
        if Instant::now() >= someone_else_holds_it_by_now {
            tracing::warn!(
                instance_name,
                "the shared-instance name is held by someone else; a start under it will refuse"
            );
            return;
        }
        std::thread::sleep(BETWEEN_ASKS);
    }
}

#[cfg(target_os = "linux")]
fn taken(instance_name: &str) -> bool {
    use std::os::linux::net::SocketAddrExt as _;
    let named = format!("rns/{instance_name}");
    let Ok(addr) = std::os::unix::net::SocketAddr::from_abstract_name(named.as_bytes()) else {
        return false;
    };
    std::os::unix::net::UnixListener::bind_addr(&addr).is_err()
}

#[cfg(all(unix, not(target_os = "linux")))]
fn taken(instance_name: &str) -> bool {
    let named = format!("rns/{instance_name}").replace('/', "-");
    std::env::temp_dir()
        .join(format!("leviculum-{named}"))
        .exists()
}

#[cfg(not(unix))]
fn taken(_instance_name: &str) -> bool {
    false
}

#[cfg(test)]
mod tests;
