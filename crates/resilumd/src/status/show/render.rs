mod sections;

use std::fmt::Write as _;
use std::time::Duration;

use resilum_core::status::NodeStatus;

use super::A_SNAPSHOT_THIS_OLD_IS_NOT_A_LIVE_NODE;
use super::paint::{bold, dimmed, green, red, yellow};

pub fn all_of_it(status: &NodeStatus, age: Option<Duration>, asked: &super::Asked) -> String {
    let mut out = String::new();
    who_we_are(&mut out, status, age);
    sections::what_has_moved(&mut out, status, asked.links);
    sections::where_we_sit(&mut out, status, asked.map);
    sections::what_carries_us(&mut out, status, asked.interfaces);
    sections::who_we_hold(&mut out, status, asked.links);
    out
}

const BETWEEN_FIELDS: &str = "   ";

fn side_by_side(fields: &[String]) -> String {
    fields.join(BETWEEN_FIELDS)
}

fn heading(text: &str) -> String {
    format!("\n{}\n", bold(text))
}

fn who_we_are(out: &mut String, status: &NodeStatus, age: Option<Duration>) {
    let _ = writeln!(
        out,
        "resilum {}  {}  {}",
        bold(&status.version),
        dimmed(status.identity_hash.as_deref().unwrap_or("unknown")),
        how_it_is(status.running, age),
    );
}

fn how_it_is(running: bool, age: Option<Duration>) -> String {
    match age {
        Some(age) if age <= A_SNAPSHOT_THIS_OLD_IS_NOT_A_LIVE_NODE => {
            if running {
                green("● running")
            } else {
                red("○ stopped")
            }
        }
        Some(age) => yellow(&format!("○ silent for {}", how_long(age))),
        None => yellow("○ never heard from"),
    }
}

fn how_long(age: Duration) -> String {
    let seconds = age.as_secs();
    match seconds {
        s if s < 90 => format!("{s}s"),
        s if s < 5400 => format!("{}m", s / 60),
        s => format!("{}h", s / 3600),
    }
}
