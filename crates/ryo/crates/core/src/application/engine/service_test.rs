//! The `engine` service's cases: the compilation it holds compiles no rules to no queries.

use super::service::Compilation;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution::inbound::use_cases::compile_rules::interface::CompileRulesUseCase as _;
use ryo_execution::inbound::use_cases::compile_rules::types::CompileRequest;
use ryo_execution::outbound::query_language::fixtures::fake::QueryLanguageFake;

/// What a row of the cases asks of the service.
type Input = ();
/// Whether the service answered as the row expects.
type Value = bool;
/// What a row of the cases expects when it is refused.
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("no_rules_compile_to_no_queries_and_no_error"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(true),
}];

/// Compiles no rules through the engine's compilation over the fake query language.
fn subject(_: Input) -> Result<Value, Failure> {
    let compilation = Compilation::new(
        std::sync::Arc::new(QueryLanguageFake::default()),
        std::sync::Arc::new(ryo_execution_runtime::outbound::digest::fixtures::fake::DigestFake),
    );
    Ok(compilation
        .compile_rules(CompileRequest {
            rules: &[],
            hosted: &[],
            vocabularies: &ryo_values::domain::descriptions::types::KindVocabularies(Vec::new()),
        })
        .expect("no rules compile")
        .plans()
        .is_empty())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
