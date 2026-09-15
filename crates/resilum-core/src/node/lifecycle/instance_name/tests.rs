use std::time::{Duration, Instant};

use super::{taken, wait_until_free};

fn a_name() -> String {
    format!("resilum-test-{}-{:?}", std::process::id(), Instant::now())
}

#[test]
fn a_name_nobody_holds_is_free() {
    assert!(!taken(&a_name()));
}

#[cfg(target_os = "linux")]
#[test]
fn a_name_held_by_a_listener_is_taken() {
    use std::os::linux::net::SocketAddrExt as _;

    let name = a_name();
    let addr = std::os::unix::net::SocketAddr::from_abstract_name(format!("rns/{name}").as_bytes())
        .expect("an abstract name");
    let held = std::os::unix::net::UnixListener::bind_addr(&addr).expect("to hold it");

    assert!(taken(&name));
    drop(held);
    assert!(!taken(&name));
}

#[cfg(target_os = "linux")]
#[test]
fn waiting_returns_once_the_holder_lets_go() {
    use std::os::linux::net::SocketAddrExt as _;

    let name = a_name();
    let addr = std::os::unix::net::SocketAddr::from_abstract_name(format!("rns/{name}").as_bytes())
        .expect("an abstract name");
    let held = std::os::unix::net::UnixListener::bind_addr(&addr).expect("to hold it");
    let letting_go =
        resilum_tasks::a_thread_of_its_own("a test holder letting the name go", || {
            std::thread::sleep(Duration::from_millis(120));
            drop(held);
        })
        .expect("a thread to hold it");

    let began = Instant::now();
    wait_until_free(&name);

    assert!(
        began.elapsed() >= Duration::from_millis(100),
        "returned early"
    );
    assert!(!taken(&name));
    letting_go.join().expect("the holder thread");
}
