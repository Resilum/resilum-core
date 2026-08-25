use std::io::Write;
use std::os::fd::AsRawFd;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::*;

fn a_socket_pair() -> (
    std::os::unix::net::UnixStream,
    std::os::unix::net::UnixStream,
) {
    std::os::unix::net::UnixStream::pair().expect("a socket pair")
}

#[test]
fn a_carrier_with_traffic_wins() {
    let (mut writer, reader) = a_socket_pair();
    let wake = Wake::new().expect("a pipe");

    writer.write_all(b"x").expect("to send a byte");

    assert!(matches!(
        wake.wait_for_carrier_or_a_raise(&[reader.as_raw_fd()])
            .expect("no poll error"),
        Ready::Carrier
    ));
}

#[test]
fn a_silent_carrier_still_lets_the_thread_leave() {
    let (_writer, reader) = a_socket_pair();
    let wake = Arc::new(Wake::new().expect("a pipe"));
    let raiser = wake.clone();

    let waiting = std::thread::spawn(move || {
        let started = Instant::now();
        let ready = wake
            .wait_for_carrier_or_a_raise(&[reader.as_raw_fd()])
            .expect("no poll error");
        (ready, started.elapsed())
    });
    std::thread::sleep(Duration::from_millis(50));
    raiser.raise();

    let (ready, waited) = waiting.join().expect("the thread above");
    assert!(matches!(ready, Ready::Woken));
    assert!(waited < Duration::from_secs(5), "woken after {waited:?}");
}

#[test]
fn raising_before_the_wait_is_not_missed() {
    let (_writer, reader) = a_socket_pair();
    let wake = Wake::new().expect("a pipe");

    wake.raise();

    assert!(matches!(
        wake.wait_for_carrier_or_a_raise(&[reader.as_raw_fd()])
            .expect("no poll error"),
        Ready::Woken
    ));
    assert!(wake.raised());
}
