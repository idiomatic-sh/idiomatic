//! The globset adapter of the matching port runs the contract, and refuses a pattern that is no glob.

use super::adapter::MatchingGlobsetAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::selection::types::{PathMatch, PathPattern};
use crate::domain::violations::types::RepoPath;
use crate::outbound::matching::errors::MatchingError;
use crate::outbound::matching::fixtures::contract::check;
use crate::outbound::matching::interface::MatchingPort as _;

/// The pattern asked of, or none for the contract.
type Input = Option<&'static str>;
/// Whether the path matched.
type Value = PathMatch;
type Failure = MatchingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_adapter_keeps_the_contract"),
        word: Word::Succeeds,
        input: || None,
        expect: Expect::Value(PathMatch::Matched),
    },
    Case {
        name: CaseName("an_unclosed_class_is_no_glob"),
        word: Word::Refuses,
        input: || Some("src/[a"),
        expect: Expect::Refusal(|actual| {
            if actual
                == &(MatchingError::Pattern {
                    pattern: "src/[a".into(),
                    reason: "unclosed character class; missing ']'".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
];

fn subject(pattern: Input) -> Result<Value, Failure> {
    let Some(pattern) = pattern else {
        check(&MatchingGlobsetAdapter);
        return Ok(PathMatch::Matched);
    };
    // The refusal row's selector checks the error, its pattern and reason included.
    MatchingGlobsetAdapter.matches(&[PathPattern(pattern.into())], &RepoPath("src/a".into()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
