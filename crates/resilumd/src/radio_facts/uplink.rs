use std::path::Path;

const ROUTES: &str = "/proc/net/route";
const NOWHERE_IN_PARTICULAR: &str = "00000000";

pub fn this_host_can_reach_the_world() -> bool {
    a_default_route_in(Path::new(ROUTES))
}

fn a_default_route_in(routes: &Path) -> bool {
    let Ok(table) = std::fs::read_to_string(routes) else {
        return false;
    };
    table
        .lines()
        .skip(1)
        .filter_map(|line| {
            let mut columns = line.split_whitespace();
            let interface = columns.next()?;
            let destination = columns.next()?;
            Some((interface, destination))
        })
        .any(|(interface, destination)| destination == NOWHERE_IN_PARTICULAR && interface != "lo")
}

#[cfg(test)]
mod tests;
