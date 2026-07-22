//! Default network anchors.

// leviculum ignores `bootstrap_only`, so these currently stay connected.
pub(crate) const PUBLIC_ANCHORS: &[&str] = &[
    "istanbul.reserve.network:9034",
    "vjs.hu:5858",
    "rns.beleth.net:4242",
    "dfw.us.g00n.cloud:6969",
    "use.inertia.chat:4242",
];

pub(crate) const YGG_ANCHORS: &[&str] = &[
    "[200:3953:999b:282e:e526:bcd2:c329:31a]:4343",
    "[203:f54a:ffa2:650d:df9d:1473:5228:94dc]:43434",
    "[201:e73a:61ee:ca68:4bc5:99d5:fd70:ded1]:4343",
];

pub(crate) const DEFAULT_LISTEN: &str = "[::]:4242";
pub(crate) const DISCOVERY_NAME: &str = "resilum";
pub(crate) const NETWORK_IDENTITY_FILE: &str = "network_identity";
