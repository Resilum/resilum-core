//! End-to-end: one egress node registers two services, and each inbound link is
//! routed to its own service's target (by the destination_hash on the link).

mod harness;
mod probe;
#[path = "../support/mod.rs"]
mod support;

use harness::{services, spawn_tagged_echo, start_client, start_egress};
use probe::probe_service;

fn temp_dir(tag: &str) -> std::path::PathBuf {
    support::temp_dir("ms", tag)
}

#[test]
fn each_service_is_routed_to_its_own_target() {
    let echo_a = spawn_tagged_echo(b'A');
    let echo_b = spawn_tagged_echo(b'B');

    let dir = temp_dir("egress");
    let (mut egress, egress_port) = start_egress(services(echo_a, echo_b), &dir);

    let dir_c = temp_dir("client");
    let mut client = start_client(&dir_c, egress_port);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let engine = client.engine().expect("engine");
    let events = client.events().clone();
    let (a, b) = rt.block_on(async {
        let a = probe_service(&engine, &events, "svc-a").await;
        let b = probe_service(&engine, &events, "svc-b").await;
        (a, b)
    });

    client.stop().ok();
    egress.stop().ok();
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(dir_c);

    assert_eq!(a, b'A', "svc-a reached the A target");
    assert_eq!(b, b'B', "svc-b reached the B target");
}
