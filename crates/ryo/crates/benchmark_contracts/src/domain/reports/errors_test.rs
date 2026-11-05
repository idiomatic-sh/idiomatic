//! Boundary failures retain their original exact reason and source category.
use super::errors::ReportsError;
#[test]
fn invalid_observation_is_not_an_empty_success_and_preserves_its_reason() {
    let error = ReportsError::Invalid("coverage is missing".into());
    assert!(matches!(&error,ReportsError::Invalid(reason) if reason=="coverage is missing"));
    assert_eq!(error.to_string(), "coverage is missing");
}
