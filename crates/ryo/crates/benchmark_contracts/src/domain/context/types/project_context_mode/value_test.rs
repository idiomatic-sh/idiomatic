//! Isolation is the default of an undeclared context mode.
use super::value::ProjectContextMode;
#[test]
fn undeclared_context_mode_defaults_to_isolation() {
    assert_eq!(ProjectContextMode::default(), ProjectContextMode::Isolated);
}
