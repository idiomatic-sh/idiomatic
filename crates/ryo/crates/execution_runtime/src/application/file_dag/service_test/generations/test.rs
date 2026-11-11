//! Two file generation owners cannot reuse each other's results: each file's generation consults its own provider and keeps its output.
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
