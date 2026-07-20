//! Baked-in default network, matching the project's shipped
//! network config: the anchors and discovery settings a bare node joins the
//! global mesh through.

/// Public clearnet anchors. In these are `bootstrap_only` (dropped once
/// paths are learned); leviculum does not implement that flag yet, so we render
/// it for config parity but the connections currently stay up.
pub(crate) const PUBLIC_ANCHORS: &[&str] = &[
    "istanbul.reserve.network:9034",
    "vjs.hu:5858",
    "rns.beleth.net:4242",
    "dfw.us.g00n.cloud:6969",
    "use.inertia.chat:4242",
];

/// Persistent transport anchors reached over Yggdrasil.
pub(crate) const YGG_ANCHORS: &[&str] = &[
    "[200:3953:999b:282e:e526:bcd2:c329:31a]:4343",
    "[203:f54a:ffa2:650d:df9d:1473:5228:94dc]:43434",
    "[201:e73a:61ee:ca68:4bc5:99d5:fd70:ded1]:4343",
];

pub(crate) const DEFAULT_LISTEN: &str = "[::]:4242";
pub(crate) const DISCOVERY_NAME: &str = "resilum";
