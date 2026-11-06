//! Production inbound semantic controls with positive and refusal cases.

use super::errors::MatchingError;
use super::fixtures::subject::{MatchingInput, MatchingValue, answer};
type Input = (MatchingInput, MatchingValue);
type Value = ();
use super::use_case::MatchingUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run};

type Failure = MatchingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("text_glob_crosses_separator"),
        word: Word::Succeeds,
        input: || {
            (
                MatchingInput::TextGlob("src/*.rs", "src/a/b.rs"),
                MatchingValue::Match(true),
            )
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("text_glob_keeps_unicode_question_semantics"),
        word: Word::Succeeds,
        input: || {
            (
                MatchingInput::TextGlob("a?c", "aéc"),
                MatchingValue::Match(true),
            )
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("text_glob_negative"),
        word: Word::Succeeds,
        input: || {
            (
                MatchingInput::TextGlob("a?c", "aééc"),
                MatchingValue::Match(false),
            )
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("text_glob_literal_malformed_syntax"),
        word: Word::Succeeds,
        input: || {
            (
                MatchingInput::TextGlob("{a", "{a"),
                MatchingValue::Match(true),
            )
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("regex_match"),
        word: Word::Succeeds,
        input: || {
            (
                MatchingInput::Regex("^ok$", "ok"),
                MatchingValue::Match(true),
            )
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("regex_refusal"),
        word: Word::Refuses,
        input: || {
            (
                MatchingInput::Regex("[", "text"),
                MatchingValue::Match(false),
            )
        },
        expect: Expect::Refusal(|error| {
            crate::selects!(
                matches!(error, MatchingError::InvalidRegex { reason } if !reason.is_empty()),
            )
        }),
    },
    Case {
        name: CaseName("glob_path_semantics"),
        word: Word::Succeeds,
        input: || {
            (
                MatchingInput::Globs(&["*.rs", "**/*.rs"], "src/main.rs"),
                MatchingValue::Hits(vec![1].into()),
            )
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("glob_refusal"),
        word: Word::Refuses,
        input: || (MatchingInput::Parse("["), MatchingValue::Match(false)),
        expect: Expect::Refusal(|error| {
            crate::selects!(
                matches!(error, MatchingError::BadGlob { glob, reason } if glob == "[" && !reason.is_empty()),
            )
        }),
    },
];

fn subject((input, expected): Input) -> Result<Value, Failure> {
    let matching = MatchingUseCaseImpl::new();
    super::fixtures::contract::check(&matching);
    let actual = answer(&matching, input)?;
    assert_eq!(actual, expected);
    Ok(())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
