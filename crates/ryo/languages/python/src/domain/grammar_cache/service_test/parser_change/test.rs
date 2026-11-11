//! A parser implementation change invalidates completed active results.

use super::fixtures::completed_active_results_are_invalidated_when_parser_implementation_changes;
use crate::cases::{Case, CaseName, Expect, Word, run};

/// Which scenario a row runs.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The `completed_active_results_are_invalidated_when_parser_implementation_changes` scenario.
    CompletedActiveResultsAreInvalidatedWhenParserImplementationChanges,
}
/// What a row is given.
type Input = Given;
/// What the scenario observed, each check in order, joined by `|`.
type Value = &'static str;
/// A scenario is never refused.
type Failure = ();

/// The rows of this group's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("completed_active_results_are_invalidated_when_parser_implementation_changes"),
    word: Word::Succeeds,
    input: || Given::CompletedActiveResultsAreInvalidatedWhenParserImplementationChanges,
    expect: Expect::Value("true|true"),
}];

/// Runs the scenario the row names.
fn subject(given: Input) -> Result<Value, Failure> {
    let observed = match given {
        Given::CompletedActiveResultsAreInvalidatedWhenParserImplementationChanges => {
            completed_active_results_are_invalidated_when_parser_implementation_changes()
        }
    };
    Ok(Box::leak(observed.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
