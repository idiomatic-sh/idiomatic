//! The omission falsifier runs the actual runtime and all other KernelSession calls.
use crate::cases::{Case, Word, run};
/// The input, value and failure of a row, as the group's fixtures state them.
type Input = super::fixtures::rows::Input;
type Value = super::fixtures::rows::Value;
type Failure = super::fixtures::rows::Failure;

/// The group's rows, stated with its fixtures.
const CASES: &[Case<Input, Value, Failure>] = super::fixtures::rows::CASES;

/// Runs one row through the scenario its fixtures build.
fn subject(input: Input) -> Result<Value, Failure> {
    super::fixtures::rows::subject(input)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn rejects() {
    run(CASES, Word::Rejects, subject);
}
