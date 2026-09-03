use std::path::PathBuf;

use super::the_way_out_in;

const HEADER: &str = "Iface\tDestination\tGateway\tFlags\tRefCnt\tUse\tMetric\tMask\n";

fn routes_named(what: &str, body: &str) -> PathBuf {
    let at = std::env::temp_dir().join(format!("resilum-routes-{}-{what}", std::process::id()));
    std::fs::write(&at, format!("{HEADER}{body}")).expect("routes");
    at
}

#[test]
fn a_host_with_a_way_out_names_the_interface_it_leaves_by() {
    let routes = routes_named(
        "a-way-out",
        "wlan0\t00000000\t0100A8C0\t0003\t0\t0\t600\t00000000\n",
    );

    assert_eq!(the_way_out_in(&routes), Some(String::from("wlan0")));
}

#[test]
fn a_wired_host_names_its_cable_and_not_its_radio() {
    let routes = routes_named(
        "over-a-cable",
        "wlan0\t0000A8C0\t00000000\t0001\t0\t0\t600\t00FFFFFF\n\
         end0\t00000000\t0100A8C0\t0003\t0\t0\t100\t00000000\n",
    );

    assert_eq!(the_way_out_in(&routes), Some(String::from("end0")));
}

#[test]
fn a_host_that_only_knows_its_own_subnet_has_no_way_out() {
    let routes = routes_named(
        "own-subnet",
        "wlan0\t0000A8C0\t00000000\t0001\t0\t0\t600\t00FFFFFF\n",
    );

    assert_eq!(the_way_out_in(&routes), None);
}

#[test]
fn a_route_to_nowhere_over_loopback_is_not_a_way_out() {
    let routes = routes_named(
        "loopback",
        "lo\t00000000\t00000000\t0003\t0\t0\t0\t00000000\n",
    );

    assert_eq!(the_way_out_in(&routes), None);
}

#[test]
fn a_table_that_cannot_be_read_leaves_the_host_without_one() {
    assert_eq!(
        the_way_out_in(std::path::Path::new("/nowhere-at-all")),
        None
    );
}
