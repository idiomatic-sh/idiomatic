//! Production inbound semantic controls with positive and refusal cases.

use super::errors::MatchingError;
use super::fixtures::contract::{MatchingInput, MatchingValue, answer};
type Input = MatchingInput;
type Value = MatchingValue;
use super::use_case::MatchingUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run};

type Failure = MatchingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("text_glob_crosses_separator"),
        word: Word::Succeeds,
        input: || Input::TextGlob("src/*.rs", "src/a/b.rs"),
        expect: Expect::Value(Value::Match(true)),
    },
    Case {
        name: CaseName("text_glob_keeps_unicode_question_semantics"),
        word: Word::Succeeds,
        input: || Input::TextGlob("a?c", "aéc"),
        expect: Expect::Value(Value::Match(true)),
    },
    Case {
        name: CaseName("text_glob_negative"),
        word: Word::Succeeds,
        input: || Input::TextGlob("a?c", "aééc"),
        expect: Expect::Value(Value::Match(false)),
    },
    Case {
        name: CaseName("text_glob_literal_malformed_syntax"),
        word: Word::Succeeds,
        input: || Input::TextGlob("{a", "{a"),
        expect: Expect::Value(Value::Match(true)),
    },
    Case {
        name: CaseName("regex_match"),
        word: Word::Succeeds,
        input: || Input::Regex("^ok$", "ok"),
        expect: Expect::Value(Value::Match(true)),
    },
    Case {
        name: CaseName("regex_refusal"),
        word: Word::Refuses,
        input: || Input::Regex("[", "text"),
        expect: Expect::Refusal(|error| {
            crate::cases::selects(
                matches!(error, MatchingError::InvalidRegex { reason } if !reason.is_empty()),
            )
        }),
    },
    Case {
        name: CaseName("glob_path_semantics"),
        word: Word::Succeeds,
        input: || Input::Globs(&["*.rs", "**/*.rs"], "src/main.rs"),
        expect: Expect::Value(Value::Hits(vec![1])),
    },
    Case {
        name: CaseName("glob_refusal"),
        word: Word::Refuses,
        input: || Input::Parse("["),
        expect: Expect::Refusal(|error| {
            crate::cases::selects(
                matches!(error, MatchingError::BadGlob { glob, reason } if glob == "[" && !reason.is_empty()),
            )
        }),
    },
];

fn subject(input: Input) -> Result<Value, Failure> {
    let matching = MatchingUseCaseImpl::new();
    super::fixtures::contract::check(&matching);
    answer(&matching, input)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
