use resilum_core::status::NodeStatus;

const EVERYTHING_BESIDE_THE_NAME_TAKES: usize = 56;
const A_TERMINAL_WE_STILL_FIT: usize = 100;
pub const WIDEST_NAME_WE_SHOW: usize = A_TERMINAL_WE_STILL_FIT - EVERYTHING_BESIDE_THE_NAME_TAKES;

pub fn name_column(status: &NodeStatus) -> usize {
    let longest = status
        .interfaces
        .iter()
        .map(|i| i.name.chars().count())
        .chain(
            status
                .links
                .iter()
                .filter_map(|l| l.interface_name.as_ref())
                .map(|name| name.chars().count()),
        )
        .max()
        .unwrap_or(0);
    longest.min(WIDEST_NAME_WE_SHOW)
}

pub fn keeping_both_ends(name: &str, column: usize) -> String {
    let width = name.chars().count();
    if width <= column || column < "…".chars().count() + 2 {
        return name.to_owned();
    }
    let tail = (column - 1) / 3;
    let head: String = name.chars().take(column - 1 - tail).collect();
    let end: String = name.chars().skip(width - tail).collect();
    format!("{head}…{end}")
}

pub fn bytes(count: u64) -> String {
    const STEP: u64 = 1024;
    match count {
        b if b < STEP => format!("{b} B"),
        b if b < STEP * STEP => format!("{:.1} kB", b as f64 / STEP as f64),
        b if b < STEP * STEP * STEP => format!("{:.1} MB", b as f64 / (STEP * STEP) as f64),
        b => format!("{:.1} GB", b as f64 / (STEP * STEP * STEP) as f64),
    }
}

pub fn shortened(hash: &str) -> String {
    keeping_both_ends(hash, 16)
}
