//! All nine real families reach the common admitted owner.
use crate::cases::{Case, Word, run};
use ryo_derived::domain::request::types::DerivedKind;
/// The input, value and failure of a row, as the group's fixtures state them.
type Input = super::fixtures::Input;
type Value = super::fixtures::Value;
type Failure = super::fixtures::Failure;

/// The group's rows, stated with its fixtures.
const CASES: &[Case<Input, Value, Failure>] = super::fixtures::CASES;

/// Runs one row through the scenario its fixtures build.
fn subject(input: Input) -> Result<Value, Failure> {
    super::fixtures::subject(input)
}

#[test]
fn succeeds() {
    let mut declared = CASES.iter().map(|case| (case.input)()).collect::<Vec<_>>();
    declared.sort_by_key(|kind| kind.ordinal());
    assert_eq!(
        declared,
        DerivedKind::ALL,
        "the independent case table must contain every authoritative family exactly once"
    );
    run(CASES, Word::Succeeds, subject);
}
