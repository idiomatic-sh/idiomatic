//! Exact byte offsets, absence and refusal through actual compiled owner execution.
use ryo_values::cases::{Case, Word, run};

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
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
