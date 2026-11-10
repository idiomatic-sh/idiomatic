//! A `harness = false` case reports its verdict and keeps it: a pass returns, a failure still fails.

use super::harness_case;

#[test]
fn a_passing_case_returns() {
    harness_case("harness::passes", || {});
}

#[test]
fn a_panicking_case_still_panics_after_its_report() {
    let outcome = std::panic::catch_unwind(|| {
        harness_case("harness::fails", || panic!("the planted failure"));
    });
    assert!(
        outcome.is_err(),
        "a failed case resumes its panic after the report"
    );
}
