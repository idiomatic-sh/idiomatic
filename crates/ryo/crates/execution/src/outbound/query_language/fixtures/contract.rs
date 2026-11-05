//! What every `query_language` adapter must pass.

use ryo_values::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::outbound::query_language::errors::QueryLanguageError;
use crate::outbound::query_language::interface::QueryLanguagePort;
use ryo_values::domain::plan_parts::types::QueryText;
use ryo_values::domain::program::types::QueryBundle;

/// What a row of the contract is given: query text, which every implementation validates and compiles alike.
type Input = QueryBundle;
/// Whether the text was validated and compiled.
type Value = bool;
/// What a row of the contract expects when the text is refused.
type Failure = QueryLanguageError;

/// A query every implementation compiles.
const A_QUERY: &str =
    "let functions = select * from \"function\"; emit functions subject \"file\" key(\"span\");";
/// A text that is no query, which every implementation refuses.
const TEXT_THAT_REFUSE_AND_IS_NOT_A_QUERY: &str = "refuse this";

/// The contract's rows: a name, one case word, an input and the expected value or error variant. Every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_query_compiles"),
        word: Word::Succeeds,
        input: || QueryBundle::from(QueryText(A_QUERY.into())),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("text_that_is_not_a_query_is_refused"),
        word: Word::Refuses,
        input: || QueryBundle::from(QueryText(TEXT_THAT_REFUSE_AND_IS_NOT_A_QUERY.into())),
        expect: Expect::Refusal(|error| {
            selects(matches!(error, QueryLanguageError::Refused { .. }))
        }),
    },
];

/// Runs the contract's rows against one implementation: the fake, or an adapter.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(adapter: &impl QueryLanguagePort) {
    let subject = |text| adapter.compile(&text, &[], &[]).map(|_| true);
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
    let validated = |text| adapter.validate(&text, &[], &[]).map(|_| true);
    run(CASES, Word::Succeeds, validated);
    run(CASES, Word::Refuses, validated);
}
