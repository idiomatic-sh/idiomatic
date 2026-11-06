//! The `globset` adapter of the matching port: the port's contract rows run through it, then the cases specific to it.

use super::adapter::PatternMatchingGlobsetAdapter;
use crate::cases::{Matched, Case, CaseName, Expect, Word, run, selects};
use crate::domain::paths::types::EntryName;
use crate::domain::paths::types::Matching;
use crate::domain::targets::types::Pattern;
use crate::outbound::pattern_matching::errors::PatternMatchingError;
use crate::outbound::pattern_matching::fixtures::contract::check;
use crate::outbound::pattern_matching::interface::PatternMatchingPort as _;

/// The pattern, and the path it is tried on.
type Input = (&'static str, &'static str);
/// Whether the path matches.
type Value = Matching;
type Failure = PatternMatchingError;

/// The rows of what is specific to this adapter.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_question_mark_stands_for_one_character"),
        word: Word::Succeeds,
        input: || ("crates/co?e", "crates/core"),
        expect: Expect::Value(Matching::Matches),
    },
    Case {
        name: CaseName("a_class_stands_for_one_of_its_characters"),
        word: Word::Succeeds,
        input: || ("crates/[cd]ore", "crates/core"),
        expect: Expect::Value(Matching::Matches),
    },
    Case {
        name: CaseName("a_class_that_is_not_closed_is_refused"),
        word: Word::Refuses,
        input: || ("crates/[c", "crates/core"),
        expect: Expect::Refusal(|error| {
            selects(Matched(
                matches!(error, PatternMatchingError::Invalid { pattern, reason }
                if &*pattern.0 == "crates/[c" && &*reason.0 == "unclosed character class; missing ']'")),
            )
        }),
    },
];

fn subject((pattern, text): Input) -> Result<Value, Failure> {
    PatternMatchingGlobsetAdapter.matches(&Pattern(pattern.into()), &EntryName(text.into()))
}

#[test]
fn succeeds() {
    check(&PatternMatchingGlobsetAdapter);
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
