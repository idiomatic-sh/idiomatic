//! Regular-expression boundary, error and compilation-reuse cases.

use super::service::matches;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::types::Verdict;
use crate::domain::table_texts::types::{Datum, TextView};

type Input = (&'static str, &'static str);
type Value = Verdict;
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("case_insensitive_directive_word_boundary"),
        word: Word::Succeeds,
        input: || (" NOQA: E722, F401", "(?i)^ *noqa: *E722(?:[, ]|$)"),
        expect: Expect::Value(Verdict::Holds),
    },
    Case {
        name: CaseName("code_prefix_is_not_a_code"),
        word: Word::Succeeds,
        input: || ("noqa: E7220", "(?i)^ *noqa: *E722(?:[, ]|$)"),
        expect: Expect::Value(Verdict::Fails),
    },
    Case {
        name: CaseName("invalid_pattern_is_a_failure"),
        word: Word::Refuses,
        input: || ("anything", "["),
        expect: Expect::Refusal(|error| {
            crate::cases::selects(matches!(error, OperatingError::InvalidRegex { .. }))
        }),
    },
];

fn subject((text, pattern): Input) -> Result<Value, Failure> {
    let mut cache = super::types::Patterns::default();
    let first = matches(
        &Datum::Text(TextView(text)),
        &Datum::Text(TextView(pattern)),
        &mut cache,
    )?;
    let second = matches(
        &Datum::Text(TextView(text)),
        &Datum::Text(TextView(pattern)),
        &mut cache,
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
