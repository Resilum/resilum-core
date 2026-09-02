use std::path::PathBuf;

use super::a_default_route_in;

const HEADER: &str = "Iface\tDestination\tGateway\tFlags\tRefCnt\tUse\tMetric\tMask\n";

fn routes_holding(body: &str) -> PathBuf {
    let at = std::env::temp_dir().join(format!(
        "resilum-routes-{}-{}",
        std::process::id(),
        body.len()
    ));
    std::fs::write(&at, format!("{HEADER}{body}")).expect("routes");
    at
}

#[test]
fn a_host_with_a_way_out_says_so() {
    let routes = routes_holding("wlan0\t00000000\t0100A8C0\t0003\t0\t0\t600\t00000000\n");

    assert!(a_default_route_in(&routes));
}

#[test]
fn a_host_that_only_knows_its_own_subnet_has_no_way_out() {
    let routes = routes_holding("wlan0\t0000A8C0\t00000000\t0001\t0\t0\t600\t00FFFFFF\n");

    assert!(!a_default_route_in(&routes));
}

#[test]
fn a_route_to_nowhere_over_loopback_is_not_a_way_out() {
    let routes = routes_holding("lo\t00000000\t00000000\t0003\t0\t0\t0\t00000000\n");

    assert!(!a_default_route_in(&routes));
}

#[test]
fn a_table_that_cannot_be_read_leaves_the_host_without_one() {
    assert!(!a_default_route_in(std::path::Path::new("/nowhere-at-all")));
}
