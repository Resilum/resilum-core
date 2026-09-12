use std::path::PathBuf;

use super::the_way_out_in;

const HEADER: &str = "Iface\tDestination\tGateway\tFlags\tRefCnt\tUse\tMetric\tMask\n";

fn routes_named(body: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let at = dir.path().join("route");
    resilum_store::write_text(&at, &format!("{HEADER}{body}")).expect("routes");
    (dir, at)
}

#[test]
fn a_host_with_a_way_out_names_the_interface_it_leaves_by() {
    let (_dir, routes) = routes_named("wlan0\t00000000\t0100A8C0\t0003\t0\t0\t600\t00000000\n");

    assert_eq!(the_way_out_in(&routes), Some(String::from("wlan0")));
}

#[test]
fn a_wired_host_names_its_cable_and_not_its_radio() {
    let (_dir, routes) = routes_named(
        "wlan0\t0000A8C0\t00000000\t0001\t0\t0\t600\t00FFFFFF\n\
         end0\t00000000\t0100A8C0\t0003\t0\t0\t100\t00000000\n",
    );

    assert_eq!(the_way_out_in(&routes), Some(String::from("end0")));
}

#[test]
fn a_host_that_only_knows_its_own_subnet_has_no_way_out() {
    let (_dir, routes) = routes_named("wlan0\t0000A8C0\t00000000\t0001\t0\t0\t600\t00FFFFFF\n");

    assert_eq!(the_way_out_in(&routes), None);
}

#[test]
fn a_route_to_nowhere_over_loopback_is_not_a_way_out() {
    let (_dir, routes) = routes_named("lo\t00000000\t00000000\t0003\t0\t0\t0\t00000000\n");

    assert_eq!(the_way_out_in(&routes), None);
}

#[test]
fn a_table_that_cannot_be_read_leaves_the_host_without_one() {
    assert_eq!(
        the_way_out_in(std::path::Path::new("/nowhere-at-all")),
        None
    );
}
