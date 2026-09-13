use super::*;

#[test]
fn a_panic_reason_survives_whichever_way_it_was_raised() {
    let literal = std::panic::catch_unwind(|| panic!("a literal")).expect_err("panicked");
    assert_eq!(reason(literal), "a literal");

    let formatted = std::panic::catch_unwind(|| panic!("formatted {}", 7)).expect_err("panicked");
    assert_eq!(reason(formatted), "formatted 7");

    let odd = std::panic::catch_unwind(|| std::panic::panic_any(7u8)).expect_err("panicked");
    assert!(reason(odd).contains("neither"));
}
