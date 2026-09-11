use resilum_core::status::NodeStatus;

use super::grid::{Cell, laid_out};
use super::paint::{dimmed, red, transport};
use super::units::{bytes, keeping_both_ends, name_column, shortened};

fn heading(names: &[&str]) -> Vec<Cell> {
    names
        .iter()
        .map(|name| Cell::painted(*name, dimmed(name)))
        .collect()
}

pub fn interfaces(status: &NodeStatus) -> String {
    let column = name_column(status);
    let mut carried: Vec<_> = status.interfaces.iter().collect();
    carried.sort_by(|a, b| {
        a.discovered_via
            .cmp(&b.discovered_via)
            .then(b.rx_bytes.cmp(&a.rx_bytes))
    });
    let mut rows = vec![heading(&["via", "", "interface", "rx", "tx", "peers"])];
    for i in carried {
        let name = keeping_both_ends(&i.name, column);
        let quiet = i.online && i.rx_bytes + i.tx_bytes == 0;
        rows.push(vec![
            Cell::painted(&i.discovered_via, transport(&i.discovered_via)),
            if i.online {
                Cell::plain("up")
            } else {
                Cell::painted("down", red("down"))
            },
            if quiet {
                Cell::painted(&name, dimmed(&name))
            } else {
                Cell::plain(name)
            },
            Cell::plain(bytes(i.rx_bytes)).leaning_right(),
            Cell::plain(bytes(i.tx_bytes)).leaning_right(),
            Cell::plain(i.peer_nodes.len().to_string()).leaning_right(),
        ]);
    }
    laid_out(&rows)
}

pub fn links(status: &NodeStatus) -> String {
    let column = name_column(status);
    let mut nodes: Vec<&str> = status
        .links
        .iter()
        .map(|link| link.identity_hash.as_str())
        .collect();
    nodes.sort_unstable();
    nodes.dedup();
    let mut rows = vec![heading(&["node", "rtt", "over", "interface"])];
    for node in nodes {
        let ways: Vec<_> = status
            .links
            .iter()
            .filter(|link| link.identity_hash == node)
            .collect();
        let rtt = ways
            .iter()
            .find_map(|link| link.estimated_rtt_ms)
            .map_or_else(|| "unknown".to_owned(), |ms| format!("{ms} ms"));
        for (nth, link) in ways.iter().enumerate() {
            let named_once = nth == 0;
            rows.push(vec![
                Cell::plain(if named_once {
                    shortened(node)
                } else {
                    String::new()
                }),
                Cell::plain(if named_once {
                    rtt.clone()
                } else {
                    String::new()
                })
                .leaning_right(),
                Cell::painted(&link.transport, transport(&link.transport)),
                Cell::plain(keeping_both_ends(
                    link.interface_name.as_deref().unwrap_or("?"),
                    column,
                )),
            ]);
        }
    }
    laid_out(&rows)
}
