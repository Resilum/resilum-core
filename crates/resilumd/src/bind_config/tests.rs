use super::parsing::{parse_rns_listen, parse_ygg_listen};
use super::regions::replace_region;

#[test]
fn parse_ygg_requires_scheme() {
    assert!(parse_ygg_listen("tcp://1.2.3.4:80").is_ok());
    assert!(parse_ygg_listen("tls://[::1]:443").is_ok());
    assert!(parse_ygg_listen("1.2.3.4:80").is_err());
}

#[test]
fn parse_rns_accepts_ipv4_and_bracketed_ipv6() {
    let parsed = parse_rns_listen("0.0.0.0:4242,[::]:4243").unwrap();
    assert_eq!(parsed, vec![("0.0.0.0".into(), 4242), ("::".into(), 4243)]);
}

#[test]
fn parse_rejects_bad_port() {
    assert!(parse_rns_listen("host:0").is_err());
    assert!(parse_rns_listen("host:abc").is_err());
    assert!(parse_rns_listen("host:999999").is_err());
}

#[test]
fn replace_region_needs_open_marker() {
    assert!(
        replace_region("no markers here\n", "tag", &["body".into()])
            .unwrap()
            .is_none()
    );
}

#[test]
fn replace_region_swaps_body_between_markers() {
    let input = "\
before
# >>> resilum:managed tag
old body
# <<< resilum:managed
after
";
    let out = replace_region(input, "tag", &["new1".into(), "new2".into()])
        .unwrap()
        .unwrap();
    assert!(out.contains("new1\nnew2"));
    assert!(!out.contains("old body"));
    assert!(out.starts_with("before\n"));
    assert!(out.ends_with("after\n"));
}

#[test]
fn replace_region_rejects_unclosed() {
    let input = "# >>> resilum:managed tag\nnever closed\n";
    assert!(replace_region(input, "tag", &[]).is_err());
}
