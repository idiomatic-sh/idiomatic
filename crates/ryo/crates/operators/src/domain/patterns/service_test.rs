//! Regular-expression boundary, error and compilation-reuse cases.

use super::service::matches;
use crate::domain::patterns::errors::PatternsError;
use crate::domain::contract::types::PredicateOutcome;
use crate::domain::table_texts::types::{Datum, TextView};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};


type Input = (&'static str, &'static str);
type Value = PredicateOutcome;
type Failure = PatternsError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("case_insensitive_directive_word_boundary"),
        word: Word::Succeeds,
        input: || (" NOQA: E722, F401", "(?i)^ *noqa: *E722(?:[, ]|$)"),
        expect: Expect::Value(PredicateOutcome::Holds),
    },
    Case {
        name: CaseName("code_prefix_is_not_a_code"),
        word: Word::Succeeds,
        input: || ("noqa: E7220", "(?i)^ *noqa: *E722(?:[, ]|$)"),
        expect: Expect::Value(PredicateOutcome::Fails),
    },
    Case {
        name: CaseName("invalid_pattern_is_a_failure"),
        word: Word::Refuses,
        input: || ("anything", "["),
        expect: Expect::Refusal(|error| {
            ryo_values::selects!(matches!(error, PatternsError::InvalidRegex { .. }))
        }),
    },
];

fn subject((text, pattern): Input) -> Result<Value, Failure> {
    let mut cache = crate::domain::table_scratch::types::Patterns::default();
    let first = matches(
        &Datum::Text(TextView(text)),
        &Datum::Text(TextView(pattern)),
        &mut cache,
        crate::domain::patterns::types::PatternMatcher(
            &ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl,
        ),
    )?;
    let second = matches(
        &Datum::Text(TextView(text)),
        &Datum::Text(TextView(pattern)),
        &mut cache,
        crate::domain::patterns::types::PatternMatcher(
            &ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl,
        ),
    )?;
    assert_eq!(cache.0.len(), 1);
    assert_eq!(first, second);
    Ok(second)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
