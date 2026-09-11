use std::fmt::Write as _;

use resilum_core::status::NodeStatus;

use super::super::paint::{dimmed, transport};
use super::super::plot;
use super::super::tables;
use super::super::units::shortened;
use super::{heading, side_by_side};

pub fn what_has_moved(out: &mut String, status: &NodeStatus, links_listed: bool) {
    let up = status.interfaces.iter().filter(|i| i.online).count();
    let ask_for_links = if links_listed || status.links.is_empty() {
        String::new()
    } else {
        dimmed(" (--links)")
    };
    let _ = writeln!(
        out,
        "\n{}",
        side_by_side(&[
            format!("paths {}", status.reachable_destinations),
            format!("links {}{ask_for_links}", status.links.len()),
            format!("interfaces {up}/{} up", status.interfaces.len()),
        ])
    );
    let Some(t) = &status.transport else {
        return;
    };
    let _ = writeln!(
        out,
        "{}",
        side_by_side(&[
            format!("sent {}", t.packets_sent),
            format!("received {}", t.packets_received),
            format!("forwarded {}", t.packets_forwarded),
            format!("dropped {}", t.packets_dropped),
            format!("announces {}", t.announces_processed),
        ])
    );
}

pub fn where_we_sit(out: &mut String, status: &NodeStatus, map_wanted: bool) {
    let ours = status.coordinates.ours;
    let at = ours.position();
    let placed = status.coordinates.peers.len();
    let _ = writeln!(
        out,
        "at {:+.4} {:+.4} {:+.4}   error {:.2}   placed {placed}{}",
        at[0],
        at[1],
        at[2],
        ours.error(),
        if map_wanted { "" } else { "   (--map)" }
    );
    if !map_wanted {
        return;
    }
    out.push_str(&heading("where everyone sits"));
    for line in plot::of(status) {
        let _ = writeln!(out, "  {line}");
    }
    let _ = writeln!(out, "  {} {}", plot::US, dimmed("this node"));
    for (nth, peer) in status.coordinates.peers.iter().enumerate() {
        let ways: Vec<String> = plot::ways_to(status, &peer.identity_hash)
            .iter()
            .map(|service| transport(service))
            .collect();
        let _ = writeln!(
            out,
            "  {} {}  {:>4} ms   {}",
            plot::letter(nth),
            shortened(&peer.identity_hash),
            peer.estimated_rtt_ms,
            side_by_side(&ways)
        );
    }
}

fn services_of(status: &NodeStatus) -> Vec<&str> {
    let mut services: Vec<&str> = status
        .interfaces
        .iter()
        .map(|i| i.discovered_via.as_str())
        .collect();
    services.sort_unstable();
    services.dedup();
    services
}

pub fn what_carries_us(out: &mut String, status: &NodeStatus, listed: bool) {
    let by_service: Vec<String> = services_of(status)
        .into_iter()
        .map(|service| {
            let over_it = status
                .interfaces
                .iter()
                .filter(|i| i.discovered_via == service);
            let up = over_it.clone().filter(|i| i.online).count();
            format!("{} {up}/{}", transport(service), over_it.count())
        })
        .collect();
    let _ = writeln!(
        out,
        "carried by   {}{}",
        by_service.join("   "),
        if listed { "" } else { "   (--interfaces)" }
    );
    if !listed {
        return;
    }
    out.push_str(&heading("interfaces"));
    let _ = writeln!(out, "{}", tables::interfaces(status));
}

pub fn who_we_hold(out: &mut String, status: &NodeStatus, listed: bool) {
    if status.links.is_empty() || !listed {
        return;
    }
    out.push_str(&heading("links"));
    let _ = writeln!(out, "{}", tables::links(status));
}
