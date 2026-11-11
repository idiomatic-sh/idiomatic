//! The four readers' resets of one source's occurrences and of oversized inputs.

use super::fixtures::{
    all_four_readers_rebuild_occurrences_after_same_file_edits,
    oversized_current_inputs_do_not_become_reusable_content,
};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// Which scenario a row runs.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The `all_four_readers_rebuild_occurrences_after_same_file_edits` scenario.
    AllFourReadersRebuildOccurrencesAfterSameFileEdits,
    /// The `oversized_current_inputs_do_not_become_reusable_content` scenario.
    OversizedCurrentInputsDoNotBecomeReusableContent,
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
        name: CaseName("all_four_readers_rebuild_occurrences_after_same_file_edits"),
        word: Word::Succeeds,
        input: || Given::AllFourReadersRebuildOccurrencesAfterSameFileEdits,
        expect: Expect::Value(
            "8|true|8|true|8|true|8|1|19|true|19|true|19|true|19|1|8|true|8|true|8|true|8|1|8|true|8|true|8|true|8|1",
        ),
    },
    Case {
        name: CaseName("oversized_current_inputs_do_not_become_reusable_content"),
        word: Word::Succeeds,
        input: || Given::OversizedCurrentInputsDoNotBecomeReusableContent,
        expect: Expect::Value("true|true|true|true|true|true|true|true|true|true|true|true"),
    },
];

/// Runs the scenario the row names.
fn subject(given: Input) -> Result<Value, Failure> {
    let observed = match given {
        Given::AllFourReadersRebuildOccurrencesAfterSameFileEdits => {
            all_four_readers_rebuild_occurrences_after_same_file_edits()
        }
        Given::OversizedCurrentInputsDoNotBecomeReusableContent => {
            oversized_current_inputs_do_not_become_reusable_content()
        }
    };
    Ok(Box::leak(observed.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
