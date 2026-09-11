use super::{Cell, laid_out};

fn starts_at(line: &str, text: &str) -> Option<usize> {
    line.find(text)
}

#[test]
fn a_painted_cell_is_padded_by_what_it_shows_not_by_its_escapes() {
    let painted = format!("\u{1b}[35m{}\u{1b}[39m", "tor");
    let rows = vec![
        vec![Cell::painted("tor", painted), Cell::plain("first")],
        vec![Cell::plain("covert_icmp"), Cell::plain("second")],
    ];

    let laid = laid_out(&rows);

    let lines: Vec<&str> = laid.lines().collect();
    assert_eq!(
        starts_at(lines[0], "first").map(|at| at - "\u{1b}[35m\u{1b}[39m".len()),
        starts_at(lines[1], "second"),
        "{laid:?}"
    );
}

#[test]
fn a_column_is_as_wide_as_its_widest_cell() {
    let rows = vec![
        vec![Cell::plain("a"), Cell::plain("x")],
        vec![Cell::plain("aaaaa"), Cell::plain("y")],
    ];

    let laid = laid_out(&rows);

    let lines: Vec<&str> = laid.lines().collect();
    assert_eq!(
        starts_at(lines[0], "x"),
        starts_at(lines[1], "y"),
        "{laid:?}"
    );
}

#[test]
fn a_number_can_lean_on_the_right_edge_of_its_column() {
    let rows = vec![
        vec![Cell::plain("7").leaning_right(), Cell::plain("x")],
        vec![Cell::plain("12345").leaning_right(), Cell::plain("y")],
    ];

    let laid = laid_out(&rows);

    assert!(laid.starts_with("    7"), "{laid:?}");
}

#[test]
fn nothing_trails_a_row_into_empty_space() {
    let rows = vec![
        vec![Cell::plain("wide-one"), Cell::plain("x")],
        vec![Cell::plain("a"), Cell::plain("y")],
    ];

    let laid = laid_out(&rows);

    assert!(laid.lines().all(|line| line == line.trim_end()), "{laid:?}");
}
