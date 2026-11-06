//! The `query_language` port's fake runs the port's contract, and refuses the text it is told to.

use super::super::contract::check;
use super::{QueryLanguageFake, REFUSED};
use crate::outbound::query_language::errors::QueryLanguageError;
use crate::outbound::query_language::interface::QueryLanguagePort as _;
use ryo_values::cases::{Case, CaseName, Expect, Word, run, selects};
use ryo_values::domain::plan_parts::types::QueryText;

/// What a row of the cases asks of the fake.
type Input = QueryText;
/// Whether the text compiled.
type Value = bool;
type Failure = QueryLanguageError;

/// The rows of the fake's own cases: a name, one case word, an input and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("text_the_fake_accepts_compiles"),
        word: Word::Succeeds,
        input: || QueryText("accept this".into()),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("text_that_contains_refuse_is_refused"),
        word: Word::Refuses,
        input: || QueryText(REFUSED.into()),
        expect: Expect::Refusal(|error| {
            selects(matches!(error, QueryLanguageError::Refused { .. }))
        }),
    },
];

/// Calls the fake on one input.
fn subject(text: &Input) -> Result<Value, Failure> {
    QueryLanguageFake::default()
        .compile(
            &ryo_values::domain::program::types::QueryBundle::from(text.clone()),
            &[],
            &[],
        )
        .map(|_| true)
}

#[test]
fn succeeds() {
    check(&QueryLanguageFake::default());
    run(CASES, Word::Succeeds, |text| subject(&text));
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, |text| subject(&text));
}
