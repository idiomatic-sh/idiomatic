//! What every `syntax` adapter must pass.

use super::super::errors::SyntaxError;
use super::super::interface::SyntaxPort;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use ryo_values::domain::program::types::QueryView;

/// What a row of the contract is given: the query text.
type Input = &'static str;
/// What a row expects when the text is read: how many `let` steps it has.
type Value = usize;
/// What a row expects when the text is refused.
type Failure = SyntaxError;

/// A program of one step that emits it: text every syntax must read.
const ONE_STEP: &str =
    "let functions = select * from \"function\"; emit functions subject \"file\" key(\"span\");";
/// A program with no `emit`: text every syntax must refuse.
const NO_EMIT: &str = "let functions = select * from \"function\";";

/// The contract's rows: a name, one case word, an input and the expected value or error variant. Every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("reads_a_query_with_one_step"),
        word: Word::Succeeds,
        input: || ONE_STEP,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_query_without_emit_is_unreadable"),
        word: Word::Refuses,
        input: || NO_EMIT,
        expect: Expect::Refusal(|error| selects(matches!(error, SyntaxError::Unreadable { .. }))),
    },
];

/// Runs the contract's rows against one implementation, the fake or an adapter, and checks the grammar it answers.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(adapter: &impl SyntaxPort) {
    let subject = |text| {
        adapter
            .read(QueryView(text))
            .map(|program| program.steps.len())
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
    let grammar = adapter.grammar();
    let blank = grammar
        .0
        .iter()
        .filter(|production| {
            production.name.0.trim().is_empty()
                || production.form.0.trim().is_empty()
                || production.example.0.trim().is_empty()
        })
        .count();
    assert!(
        !grammar.0.is_empty() && blank == 0,
        "the grammar has productions, each with a name, a form and an example"
    );
}
