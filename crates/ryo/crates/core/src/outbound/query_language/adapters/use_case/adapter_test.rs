//! The `use_case` adapter of the `query_language` port: the core's contract rows run through it, then the cases specific to it.

use std::sync::Arc;

use ryo_query_language::inbound::use_cases::compile::use_case::CompileUseCaseImpl;
use ryo_query_language::inbound::use_cases::typecheck::use_case::TypecheckUseCaseImpl;

use super::adapter::QueryLanguageUseCaseAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::outbound::query_language::errors::QueryLanguageError;
use crate::outbound::query_language::fixtures::contract::CASES as CONTRACT;
use crate::outbound::query_language::interface::QueryLanguagePort as _;
use ryo_values::domain::plan_parts::types::QueryText;

type Input = QueryText;
type Value = bool;
type Failure = QueryLanguageError;

/// The rows of what is specific to this adapter: a name, one case word, an input from the fixtures or a constructor, and the expected
/// value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_query_whose_step_has_another_name_compiles"),
        word: Word::Succeeds,
        input: || {
            QueryText(
                "let found = select * from \"function\"; emit found subject \"file\" key(\"span\");"
                    .into(),
            )
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("an_empty_text_is_refused"),
        word: Word::Refuses,
        input: || QueryText("".into()),
        expect: Expect::Refusal(|error| {
            selects(matches!(error, QueryLanguageError::Refused { .. }))
        }),
    },
];

/// Calls the adapter on one input, for the contract's rows and for its own.
fn subject(text: &Input) -> Result<Value, Failure> {
    let compile = Arc::new(CompileUseCaseImpl::new(
        Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
        Arc::new(ryo_query_language::outbound::catalog::factory::language()),
    ));
    let typecheck = Arc::new(TypecheckUseCaseImpl::new(
        Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
        Arc::new(ryo_query_language::outbound::catalog::factory::language()),
    ));
    QueryLanguageUseCaseAdapter::new(compile, typecheck)
        .compile(text, &[])
        .map(|_| true)
}

/// The rows of the examples the operators print: no input, and the count of examples that do not compile as the value.
const EXAMPLES: &[Case<(), usize, Failure>] = &[Case {
    name: CaseName("every_operator_example_compiles_as_it_is_printed"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(0),
}];

/// How many of the examples the operators print do not compile.
fn uncompiled() -> usize {
    crate::domain::operator::constants::OPERATORS
        .iter()
        .filter(|operator| {
            let described = crate::domain::operator::constants::describe(**operator);
            matches!(
                subject(&QueryText(described.example.0.clone())),
                Err(QueryLanguageError::Refused { .. })
            )
        })
        .count()
}

#[test]
fn succeeds() {
    run(CONTRACT, Word::Succeeds, |text| subject(&text));
    run(CASES, Word::Succeeds, |text| subject(&text));
    run(EXAMPLES, Word::Succeeds, |()| Ok(uncompiled()));
}

#[test]
fn refuses() {
    run(CONTRACT, Word::Refuses, |text| subject(&text));
    run(CASES, Word::Refuses, |text| subject(&text));
}
