//! Render Yggdrasil and Reticulum managed regions from listen env values.

use super::parsing::{parse_rns_listen, parse_ygg_listen};
use super::regions::replace_region;

pub(super) fn render_ygg(text: &str, value: &str) -> Result<Option<String>, String> {
    let uris = parse_ygg_listen(value)?;
    let body: Vec<String> = uris.iter().map(|u| format!(r#"    "{u}""#)).collect();
    replace_region(text, "ygg-public-listen", &body)
}

pub(super) fn render_rns(text: &str, value: &str) -> Result<Option<String>, String> {
    let binds = parse_rns_listen(value)?;
    let mut body: Vec<String> = Vec::new();
    for (idx, (host, port)) in binds.iter().enumerate() {
        if !body.is_empty() {
            body.push(String::new());
        }
        let name = if binds.len() == 1 {
            "Public TCP listener".to_string()
        } else {
            format!("Public TCP listener {}", idx + 1)
        };
        let discoverable = if idx == 0 { "yes" } else { "no" };
        body.extend([
            format!("  [[{name}]]"),
            "    type = TCPServerInterface".into(),
            "    enabled = yes".into(),
            format!("    listen_ip = {host}"),
            format!("    listen_port = {port}"),
            format!("    discoverable = {discoverable}"),
            "    discovery_name = resilum".into(),
            "    mode = gateway".into(),
        ]);
    }
    replace_region(text, "rns-public-listen", &body)
}
