//! The steps of the `typecheck` feature: each a cucumber expression over the world, one to six lines, asserting the part's own results.

use cucumber::{given, then, when};

use crate::Behavior;
use std::sync::Arc;

use ryo_query_language::inbound::use_cases::typecheck::interface::TypecheckUseCase as _;
use ryo_query_language::inbound::use_cases::typecheck::use_case::TypecheckUseCaseImpl;
use ryo_values::domain::plan_parts::types::QueryText;
use ryo_values::domain::plans::types::QueryRequest;

/// Names the scene the next `typecheck` step runs.
#[given(expr = "the typecheck input {string}")]
fn given_input(world: &mut Behavior, value: String) {
    world.input = value;
}

/// Runs `typecheck` over the scene the input names, keeping what it answered.
#[when(expr = "typecheck runs")]
fn runs(world: &mut Behavior) {
    let text = match world.input.as_str() {
        "valid" => {
            "let functions = select * from \"function\"; emit functions subject \"file\" key(\"span\");"
        }
        "unknown_relation" => {
            "let functions = select * from \"nothing\"; emit functions subject file key(file);"
        }
        _ => "let functions = selekt * from \"function\"; emit functions subject file key(file);",
    };
    let syntax = Arc::new(ryo_query_language::outbound::syntax::factory::winnow());
    let catalog = Arc::new(ryo_query_language::outbound::catalog::factory::language());
    world.answer = match TypecheckUseCaseImpl::new(syntax, catalog).typecheck(QueryRequest {
        observations: Vec::new(),
        text: ryo_values::domain::query_tokens::types::QueryBundle::from(QueryText(text.into())),
        hosted: Vec::new(),
        modules: ryo_values::domain::program::types::ParsedModules::default(),
    }) {
        Ok(report) => format!("reads {} relations", report.relations.len()),
        Err(error) => error.to_string(),
    };
}

/// Asserts the last `typecheck` step's answer.
#[then(expr = "typecheck answers {string}")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "cucumber parses a captured `{string}` into an owned value and passes it by value"
)]
fn answers(world: &mut Behavior, expected: String) {
    assert_eq!(world.answer, expected, "typecheck answered otherwise");
}
