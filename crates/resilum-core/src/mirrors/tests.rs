use super::Registry;
use super::payload::{Advert, pack, parse};

#[test]
fn roundtrip_advert() {
    let a = Advert::new(
        "1ed7ba443760f13d35a7681a9ca1ef1b".into(),
        vec!["resilum-core".into(), "resilum-mobile".into()],
    );
    let raw = pack(&a);
    let parsed = parse(&raw).unwrap();
    assert_eq!(parsed, a);
}

#[test]
fn rejects_short_or_non_hex_rngit_dest() {
    assert!(parse(br#"{"v":"0.0.0","rngit":"","repos":["x"]}"#).is_none());
    assert!(parse(br#"{"v":"0.0.0","rngit":"nothex","repos":["x"]}"#).is_none());
    assert!(
        parse(br#"{"v":"0.0.0","rngit":"gg7ba443760f13d35a7681a9ca1ef1b0","repos":["x"]}"#)
            .is_none()
    );
}

#[test]
fn rejects_empty_repos() {
    assert!(
        parse(br#"{"v":"0.0.0","rngit":"1ed7ba443760f13d35a7681a9ca1ef1b","repos":[]}"#).is_none()
    );
}

#[test]
fn registry_persists_and_reloads() {
    let dir = std::env::temp_dir().join(format!("resilum-mirrors-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("registry.json");
    {
        let r = Registry::new(Some(path.clone()));
        r.upsert(
            "aaaa".into(),
            "1ed7ba443760f13d35a7681a9ca1ef1b".into(),
            vec!["resilum-core".into()],
        );
    }
    let r2 = Registry::new(Some(path));
    let snap = r2.snapshot();
    assert_eq!(snap.len(), 1);
    assert_eq!(snap[0].peer, "aaaa");
    assert_eq!(snap[0].repos, vec!["resilum-core"]);
    std::fs::remove_dir_all(&dir).ok();
}
