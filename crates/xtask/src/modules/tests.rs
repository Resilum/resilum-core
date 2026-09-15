use super::put_modules_after_imports;

#[test]
fn modules_move_below_the_imports() {
    let put = put_modules_after_imports("mod a;\n\nuse std::sync::Arc;\n\nconst X: u8 = 1;\n");
    assert_eq!(
        put.as_deref(),
        Some("use std::sync::Arc;\n\nmod a;\n\nconst X: u8 = 1;\n")
    );
}

#[test]
fn a_blank_line_separates_the_imports_from_the_modules() {
    let put = put_modules_after_imports("use b::C;\nmod a;\n\nconst X: u8 = 1;\n");
    assert_eq!(
        put.as_deref(),
        Some("use b::C;\n\nmod a;\n\nconst X: u8 = 1;\n")
    );
}

#[test]
fn a_file_already_in_order_is_left_alone() {
    assert_eq!(
        put_modules_after_imports("use std::sync::Arc;\n\nmod a;\n\nconst X: u8 = 1;\n"),
        None
    );
}

#[test]
fn what_the_file_says_about_itself_stays_on_top() {
    let put = put_modules_after_imports("//! what this is\n#![allow(x)]\n\nmod a;\n\nuse b::C;\n")
        .expect("the module moves");
    assert!(put.starts_with("//! what this is\n#![allow(x)]\n"), "{put}");
}

#[test]
fn an_attribute_travels_with_the_module_it_guards() {
    let put =
        put_modules_after_imports("#[cfg(unix)]\nmod a;\n\nuse b::C;\n").expect("the module moves");
    assert_eq!(put, "use b::C;\n\n#[cfg(unix)]\nmod a;\n");
}

#[test]
fn a_module_with_a_body_is_not_a_declaration_to_move() {
    assert_eq!(
        put_modules_after_imports("mod a {\n    pub fn f() {}\n}\n\nuse b::C;\n"),
        None
    );
}

#[test]
fn the_tests_module_at_the_foot_of_the_file_stays_there() {
    assert_eq!(
        put_modules_after_imports("use b::C;\n\nfn f() {}\n\n#[cfg(test)]\nmod tests;\n"),
        None
    );
}
