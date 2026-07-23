//! Render the peer-client subprocess command from the configured template.
//! Placeholders: `{dst}`, `{server_identity}`, `{mtu}`.

pub fn render_client_command(
    template: &str,
    dst: &str,
    server_identity: &str,
    mtu: usize,
) -> String {
    template
        .replace("{dst}", dst)
        .replace("{server_identity}", server_identity)
        .replace("{mtu}", &mtu.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitutes_all_placeholders() {
        let cmd = render_client_command(
            "resilumd covert icmp client --dst {dst} --server-identity {server_identity} --mtu {mtu}",
            "203.0.113.9",
            "deadbeef",
            1400,
        );
        assert_eq!(
            cmd,
            "resilumd covert icmp client --dst 203.0.113.9 --server-identity deadbeef --mtu 1400"
        );
    }

    #[test]
    fn leaves_missing_placeholders_untouched() {
        let cmd = render_client_command("just --dst {dst}", "1.2.3.4", "aa", 1400);
        assert_eq!(cmd, "just --dst 1.2.3.4");
    }
}
