//! The `row_population` service's cases, on values alone.
use super::fixtures::{CASES, subject};
use ryo_values::cases::{Word, run};
use ryo_values::domain::query_operators::types::OperationKind;

#[test]
fn succeeds() {
    assert_eq!(
        CASES.len(),
        OperationKind::ALL.len(),
        "every registered kind has an independent expected row population"
    );
    run(CASES, Word::Succeeds, subject);
}
