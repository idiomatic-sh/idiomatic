//! The terminal index is lazy, memoized and cleared on source replacement.

use super::fixtures::is_lazy_retained_and_invalidated_between_files;
use crate::cases::{Case, CaseName, Expect, Word, run};

/// Which scenario a row runs.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The `is_lazy_retained_and_invalidated_between_files` scenario.
    IsLazyRetainedAndInvalidatedBetweenFiles,
}
/// What a row is given.
type Input = Given;
/// What the scenario observed, each check in order, joined by `|`.
type Value = &'static str;
/// A scenario is never refused.
type Failure = ();

/// The rows of this group's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("is_lazy_retained_and_invalidated_between_files"),
    word: Word::Succeeds,
    input: || Given::IsLazyRetainedAndInvalidatedBetweenFiles,
    expect: Expect::Value("Fresh|Complete|true|Fresh|true|true"),
}];

/// Runs the scenario the row names.
fn subject(given: Input) -> Result<Value, Failure> {
    let observed = match given {
        Given::IsLazyRetainedAndInvalidatedBetweenFiles => {
            is_lazy_retained_and_invalidated_between_files()
        }
    };
    Ok(Box::leak(observed.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
