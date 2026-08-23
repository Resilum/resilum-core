use super::*;

const TOR: &str = "tor";
const AN_EXIT: [u8; 1] = [1];

fn a_registry_holding_one_exit() -> CandidateRegistry {
    let registry = CandidateRegistry::default();
    registry.upsert(TOR, AN_EXIT.to_vec(), "*");
    registry
}

fn the_exit(registry: &CandidateRegistry) -> Candidate {
    registry
        .for_service(TOR)
        .pop()
        .expect("the exit put there above")
}

#[test]
fn one_silent_probe_does_not_write_off_an_exit() {
    let registry = a_registry_holding_one_exit();

    registry.record_probe(TOR, &AN_EXIT, None, 1.0);

    assert!(
        the_exit(&registry).healthy,
        "a Tor circuit is rebuilt per probe and fails often enough on its own"
    );
}

#[test]
fn an_exit_silent_three_probes_running_is_written_off() {
    let registry = a_registry_holding_one_exit();

    for round in 1..=3 {
        registry.record_probe(TOR, &AN_EXIT, None, f64::from(round));
    }

    assert!(!the_exit(&registry).healthy);
}

#[test]
fn an_answer_forgives_the_failures_before_it() {
    let registry = a_registry_holding_one_exit();
    registry.record_probe(TOR, &AN_EXIT, None, 1.0);
    registry.record_probe(TOR, &AN_EXIT, None, 2.0);

    registry.record_probe(TOR, &AN_EXIT, Some((0.1, 0.1)), 3.0);
    registry.record_probe(TOR, &AN_EXIT, None, 4.0);
    registry.record_probe(TOR, &AN_EXIT, None, 5.0);

    assert!(the_exit(&registry).healthy);
}

#[test]
fn a_single_slow_probe_does_not_become_the_exits_latency() {
    let registry = a_registry_holding_one_exit();
    registry.record_probe(TOR, &AN_EXIT, Some((0.1, 0.1)), 1.0);

    registry.record_probe(TOR, &AN_EXIT, Some((5.0, 5.0)), 2.0);

    let latency = the_exit(&registry)
        .effective_latency()
        .expect("both legs measured");
    assert!(
        latency < 3.4,
        "one slow round trip moved the estimate to {latency}s"
    );
    assert!(latency > 0.2, "and it must move at all: {latency}s");
}
