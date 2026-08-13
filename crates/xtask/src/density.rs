//! Comment lines per file, counted off the parse tree.

use tree_sitter::{Node, Parser};

/// Lines held by comments that are not `///` or `//!`: those document the API,
/// and no threshold tells a needed one from prose.
pub fn comment_lines(source: &str) -> usize {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .expect("the Rust grammar matches the tree-sitter version");
    let Some(tree) = parser.parse(source, None) else {
        return 0;
    };
    let mut cursor = tree.walk();
    let mut stack = vec![tree.root_node()];
    let mut lines = 0;
    while let Some(node) = stack.pop() {
        if is_plain_comment(node, source) {
            lines += node.end_position().row - node.start_position().row + 1;
        }
        stack.extend(node.children(&mut cursor));
    }
    lines
}

pub fn filled_lines(source: &str) -> usize {
    source.lines().filter(|l| !l.trim().is_empty()).count()
}

fn is_plain_comment(node: Node, source: &str) -> bool {
    if !matches!(node.kind(), "line_comment" | "block_comment") {
        return false;
    }
    let text = node.utf8_text(source.as_bytes()).unwrap_or_default();
    !(text.starts_with("///") || text.starts_with("//!"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doc_comments_do_not_count_but_plain_ones_do() {
        let source = "/// doc\n//! inner doc\n// plain\npub fn f() {}\n";
        assert_eq!(comment_lines(source), 1);
    }

    #[test]
    fn a_comment_inside_a_string_is_not_a_comment() {
        let source = "pub fn f() -> &'static str { \"// not a comment\" }\n";
        assert_eq!(comment_lines(source), 0);
    }

    #[test]
    fn a_block_comment_counts_every_line_it_spans() {
        let source = "/*\n one\n two\n*/\npub fn f() {}\n";
        assert_eq!(comment_lines(source), 4);
    }

    #[test]
    fn blank_lines_are_not_filled() {
        assert_eq!(filled_lines("a\n\n  \nb\n"), 2);
    }
}
