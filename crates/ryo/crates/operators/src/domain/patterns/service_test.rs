//! Regular-expression boundary, error and compilation-reuse cases, and the glob test's: each asked for one row and for a
//! column of texts, which answer the same.

use super::service::{glob_any, glob_any_each, matches, matches_each};
use crate::domain::contract::types::PredicateOutcome;
use crate::domain::patterns::errors::PatternsError;
use crate::domain::table_texts::types::{Datum, TextView};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};

/// A text and what it is tested against: a regular expression, or globs.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// A regular expression.
    Regex(&'static str, &'static str),
    /// One glob or several.
    Globs(&'static str, &'static [&'static str]),
}

type Input = Asked;
type Value = PredicateOutcome;
type Failure = PatternsError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("case_insensitive_directive_word_boundary"),
        word: Word::Succeeds,
        input: || Asked::Regex(" NOQA: E722, F401", "(?i)^ *noqa: *E722(?:[, ]|$)"),
        expect: Expect::Value(PredicateOutcome::Holds),
    },
    Case {
        name: CaseName("code_prefix_is_not_a_code"),
        word: Word::Succeeds,
        input: || Asked::Regex("noqa: E7220", "(?i)^ *noqa: *E722(?:[, ]|$)"),
        expect: Expect::Value(PredicateOutcome::Fails),
    },
    Case {
        name: CaseName("a_text_matches_its_one_glob"),
        word: Word::Succeeds,
        input: || Asked::Globs("test_one", &["test_*"]),
        expect: Expect::Value(PredicateOutcome::Holds),
    },
    Case {
        name: CaseName("a_text_matches_when_any_of_several_globs_does"),
        word: Word::Succeeds,
        input: || Asked::Globs("one_test", &["test_*", "*_test"]),
        expect: Expect::Value(PredicateOutcome::Holds),
    },
    Case {
        name: CaseName("a_text_no_glob_matches_does_not_hold"),
        word: Word::Succeeds,
        input: || Asked::Globs("one", &["test_*", "*_test"]),
        expect: Expect::Value(PredicateOutcome::Fails),
    },
    Case {
        name: CaseName("invalid_pattern_is_a_failure"),
        word: Word::Refuses,
        input: || Asked::Regex("anything", "["),
        expect: Expect::Refusal(|error| {
            ryo_values::selects!(matches!(error, PatternsError::InvalidRegex { .. }))
        }),
    },
];

/// The matcher of the cases.
fn matcher() -> crate::domain::patterns::types::PatternMatcher<'static> {
    crate::domain::patterns::types::PatternMatcher(
        &ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl,
    )
}

/// The glob test asked for one row, and for a column holding that one text: both answer the same.
fn globbed(text: &'static str, globs: &'static [&'static str]) -> PredicateOutcome {
    let several: Vec<ryo_values::domain::plan_parts::types::LiteralText> = globs
        .iter()
        .map(|glob| ryo_values::domain::plan_parts::types::LiteralText((*glob).into()))
        .collect();
    let right = match globs {
        [one] => Datum::Text(TextView(one)),
        _ => Datum::Texts(&several),
    };
    let row = glob_any(&Datum::Text(TextView(text)), &right, matcher());
    let views: Vec<TextView<'_>> = globs.iter().map(|glob| TextView(glob)).collect();
    let column: Vec<PredicateOutcome> =
        glob_any_each(std::iter::once(TextView(text)), &views, matcher()).collect();
    assert_eq!(column, [row]);
    row
}

fn subject(asked: Input) -> Result<Value, Failure> {
    let (text, pattern) = match asked {
        Asked::Regex(text, pattern) => (text, pattern),
        Asked::Globs(text, globs) => return Ok(globbed(text, globs)),
    };
    let mut cache = crate::domain::table_scratch::types::Patterns::default();
    let first = matches(
        &Datum::Text(TextView(text)),
        &Datum::Text(TextView(pattern)),
        &mut cache,
        matcher(),
    )?;
    let second = matches(
        &Datum::Text(TextView(text)),
        &Datum::Text(TextView(pattern)),
        &mut cache,
        matcher(),
    )?;
    assert_eq!(cache.0.len(), 1);
    assert_eq!(first, second);
    // A column holding that one text answers the same, with the one compiled expression.
    let column: Vec<PredicateOutcome> = matches_each(
        std::iter::once(TextView(text)),
        TextView(pattern),
        &mut cache,
        matcher(),
    )?
    .collect();
    assert_eq!(column, [second]);
    assert_eq!(cache.0.len(), 1);
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
