use std::path::Path;

const ROUTES: &str = "/proc/net/route";
const NOWHERE_IN_PARTICULAR: &str = "00000000";

pub fn whichever_interface_reaches_the_world() -> Option<String> {
    the_way_out_in(Path::new(ROUTES))
}

fn the_way_out_in(routes: &Path) -> Option<String> {
    let table = resilum_store::read_text(routes).ok()?;
    table
        .lines()
        .skip(1)
        .filter_map(|line| {
            let mut columns = line.split_whitespace();
            let interface = columns.next()?;
            let destination = columns.next()?;
            Some((interface, destination))
        })
        .find(|(interface, destination)| {
            *destination == NOWHERE_IN_PARTICULAR && *interface != "lo"
        })
        .map(|(interface, _)| interface.to_owned())
}

#[cfg(test)]
mod tests;
