use super::Text as _;

#[test]
fn a_line_ends_itself() {
    let mut out = String::new();
    out.line(format_args!("one {}", 1));
    out.line("two");
    assert_eq!(out, "one 1\ntwo\n");
}

#[test]
fn a_block_brings_its_own_newlines() {
    let mut out = String::new();
    out.block(format_args!("one\ntwo\n"));
    out.block("three");
    assert_eq!(out, "one\ntwo\nthree");
}
