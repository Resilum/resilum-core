use tree_sitter::{Node, Parser};

struct Item {
    from: usize,
    until: usize,
    declares_a_module: bool,
}

pub fn put_modules_after_imports(source: &str) -> Option<String> {
    let lines: Vec<&str> = source.lines().collect();
    let heading = heading_of(source);
    let modules: Vec<&Item> = heading.iter().filter(|it| it.declares_a_module).collect();
    if modules.is_empty() || modules.len() == heading.len() {
        return None;
    }
    let from = heading.first()?.from;
    let until = heading.last()?.until;
    let declares_a_module =
        |row: usize| modules.iter().any(|it| (it.from..it.until).contains(&row));

    let kept: Vec<&str> = (from..until)
        .filter(|row| !declares_a_module(*row))
        .map(|row| lines[row])
        .collect();
    let moved = (from..until).filter(|row| declares_a_module(*row));
    let mut heading_as_it_should_read: Vec<&str> = kept
        .iter()
        .skip_while(|line| line.is_empty())
        .copied()
        .collect();
    while heading_as_it_should_read
        .last()
        .is_some_and(|l| l.is_empty())
    {
        heading_as_it_should_read.pop();
    }
    heading_as_it_should_read.push("");
    heading_as_it_should_read.extend(moved.map(|row| lines[row]));
    if heading_as_it_should_read == lines[from..until] {
        return None;
    }

    let mut put: Vec<&str> = lines[..from].to_vec();
    put.extend(heading_as_it_should_read);
    put.extend(&lines[until..]);
    Some(put.join("\n") + "\n")
}

fn heading_of(source: &str) -> Vec<Item> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .expect("the Rust grammar matches the tree-sitter version");
    let Some(tree) = parser.parse(source, None) else {
        return Vec::new();
    };
    let mut cursor = tree.walk();
    let mut heading = Vec::new();
    let mut waiting: Option<usize> = None;
    for node in tree.root_node().children(&mut cursor) {
        let from = waiting.unwrap_or_else(|| node.start_position().row);
        match node.kind() {
            "attribute_item" | "inner_attribute_item" | "line_comment" | "block_comment" => {
                waiting = (!speaks_for_the_whole_file(node, source)).then_some(from);
                continue;
            }
            "use_declaration" | "mod_item" if !has_a_body(node) => heading.push(Item {
                from,
                until: node.end_position().row + 1,
                declares_a_module: node.kind() == "mod_item",
            }),
            _ => break,
        }
        waiting = None;
    }
    heading
}

fn has_a_body(node: Node) -> bool {
    node.child_by_field_name("body").is_some()
}

fn speaks_for_the_whole_file(node: Node, source: &str) -> bool {
    let text = node.utf8_text(source.as_bytes()).unwrap_or_default();
    text.starts_with("//!") || text.starts_with("#![")
}

#[cfg(test)]
mod tests;
