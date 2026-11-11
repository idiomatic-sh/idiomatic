//! Cache and demand guards for the generic decoded-format reader service.

use super::fixtures::{
    duplicate_inputs_are_parsed_once_and_retained_until_source_reset,
    malformed_results_are_cached_and_unavailable_results_mark_partial,
};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// Which scenario a row runs.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The `duplicate_inputs_are_parsed_once_and_retained_until_source_reset` scenario.
    DuplicateInputsAreParsedOnceAndRetainedUntilSourceReset,
    /// The `malformed_results_are_cached_and_unavailable_results_mark_partial` scenario.
    MalformedResultsAreCachedAndUnavailableResultsMarkPartial,
}
/// What a row is given.
type Input = Given;
/// What the scenario observed, each check in order, joined by `|`.
type Value = &'static str;
/// A scenario is never refused.
type Failure = ();

/// The rows of this group's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("duplicate_inputs_are_parsed_once_and_retained_until_source_reset"),
        word: Word::Succeeds,
        input: || Given::DuplicateInputsAreParsedOnceAndRetainedUntilSourceReset,
        expect: Expect::Value("Fresh|0|1|2|2|Some(Parsed(ParsedFormat { fields: [], summary: FormatSummary { positional_consumption: PositionalConsumption(0), mapping_present: Absent, dynamic_width: Absent, dynamic_precision: Absent } }))|1|true|2|Fresh|2"),
    },
    Case {
        name: CaseName("malformed_results_are_cached_and_unavailable_results_mark_partial"),
        word: Word::Succeeds,
        input: || Given::MalformedResultsAreCachedAndUnavailableResultsMarkPartial,
        expect: Expect::Value("1|Complete|1|true|Partial|1|Unavailable"),
    },
];

/// Runs the scenario the row names.
fn subject(given: Input) -> Result<Value, Failure> {
    let observed = match given {
        Given::DuplicateInputsAreParsedOnceAndRetainedUntilSourceReset => {
            duplicate_inputs_are_parsed_once_and_retained_until_source_reset()
        }
        Given::MalformedResultsAreCachedAndUnavailableResultsMarkPartial => {
            malformed_results_are_cached_and_unavailable_results_mark_partial()
        }
    };
    Ok(Box::leak(observed.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
