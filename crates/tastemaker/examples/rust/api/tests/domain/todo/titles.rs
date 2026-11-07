use todo_api::domain::todo::titles::{describe, normalise_title};

#[test]
fn a_title_is_trimmed_spaced_once_and_upper_case() {
    assert_eq!(normalise_title("  buy   milk "), "BUY MILK");
}

#[test]
fn a_description_names_the_title_and_its_state() {
    let described = describe("Buy milk", true, "ann", "monday", "home");
    assert!(described.starts_with("Buy milk done ann monday"), "{described}");
}
