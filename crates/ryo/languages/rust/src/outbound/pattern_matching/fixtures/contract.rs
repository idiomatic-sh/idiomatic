//! What every `pattern_matching` adapter must pass.

use super::super::errors::PatternMatchingError;
use super::super::interface::PatternMatchingPort;
use crate::cases::{Case, CaseName, Expect, Matched, Word, run, selects};
use crate::domain::paths::types::EntryName;
use crate::domain::paths::types::Matching;
use crate::domain::targets::types::Pattern;

/// What a row of the contract is given: a pattern and a path.
type Input = (&'static str, &'static str);
/// Whether the path matches.
type Value = Matching;
/// What a row of the contract expects when it is refused.
type Failure = PatternMatchingError;

/// The contract's rows: a name, one case word, an input and the expected value or error variant. Every implementation runs them.
pub(crate) const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_star_stands_for_part_of_a_segment"),
        word: Word::Succeeds,
        input: || ("crates/*", "crates/core"),
        expect: Expect::Value(Matching::Matches),
    },
    Case {
        name: CaseName("a_star_does_not_stand_for_a_slash"),
        word: Word::Succeeds,
        input: || ("crates/*", "crates/core/src"),
        expect: Expect::Value(Matching::Differs),
    },
    Case {
        name: CaseName("a_path_without_a_pattern_matches_itself"),
        word: Word::Succeeds,
        input: || ("crates/core", "crates/core"),
        expect: Expect::Value(Matching::Matches),
    },
    Case {
        name: CaseName("a_pattern_that_is_not_one_is_refused"),
        word: Word::Refuses,
        input: || ("crates/[", "crates/core"),
        expect: Expect::Refusal(|error| {
            selects(Matched(
                matches!(error, PatternMatchingError::Invalid { pattern, reason }
                if &*pattern.0 == "crates/[" && !reason.is_empty()),
            ))
        }),
    },
];

/// Runs the contract's rows against one implementation: the fake, or an adapter.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub(crate) fn check(adapter: &impl PatternMatchingPort) {
    run(CASES, Word::Succeeds, |(pattern, text)| {
        adapter.matches(&Pattern(pattern.into()), &EntryName(text.into()))
    });
    run(CASES, Word::Refuses, |(pattern, text)| {
        adapter.matches(&Pattern(pattern.into()), &EntryName(text.into()))
    });
}
