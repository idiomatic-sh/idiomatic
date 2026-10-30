//! The accepted and the refused inputs of the `globset` adapter's schemas.

use super::super::types::Matcher;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::outbound::pattern_matching::errors::PatternMatchingError;

/// The pattern, and the path it is tried on.
type Input = (&'static str, &'static str);
/// Whether the path matches.
type Value = bool;
type Failure = PatternMatchingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_pattern_with_a_star_matches_one_segment"),
        word: Word::Succeeds,
        input: || ("crates/*", "crates/core"),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_class_keeps_out_what_it_does_not_list"),
        word: Word::Succeeds,
        input: || ("crates/[ab]*", "crates/core"),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("a_pattern_that_is_not_one_is_refused"),
        word: Word::Refuses,
        input: || ("crates/[", "crates/core"),
        expect: Expect::Refusal(|error| {
            selects(
                matches!(error, PatternMatchingError::Invalid { pattern, reason }
                if &*pattern.0 == "crates/[" && &*reason.0 == "unclosed character class; missing ']'"),
            )
        }),
    },
];

fn subject((pattern, text): Input) -> Result<Value, Failure> {
    Matcher::try_from(pattern).map(|matcher| matcher.compiled.is_match(text))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
