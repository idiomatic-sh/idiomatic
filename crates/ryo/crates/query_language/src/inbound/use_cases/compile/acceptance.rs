//! The steps of the `compile` feature: each a cucumber expression over the world, one to six lines, asserting the part's own results.

use cucumber::{given, then, when};

use crate::Behavior;
use std::sync::Arc;

use ryo_query_language::inbound::use_cases::compile::interface::CompileUseCase as _;
use ryo_values::domain::plans::types::QueryRequest;
use ryo_query_language::inbound::use_cases::compile::use_case::CompileUseCaseImpl;
use ryo_values::domain::plan_parts::types::QueryText;

/// Names the scene the next `compile` step runs.
#[given(expr = "the compile input {string}")]
fn given_input(world: &mut Behavior, value: String) {
    world.input = value;
}

/// Runs `compile` over the scene the input names, keeping what it answered.
#[when(expr = "compile runs")]
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
    world.answer = match CompileUseCaseImpl::new(syntax, catalog).compile(QueryRequest {
        observations: Vec::new(),
        text: ryo_values::domain::query_tokens::types::QueryBundle::from(QueryText(text.into())),
        hosted: Vec::new(),
        modules: ryo_values::domain::program::types::ParsedModules::default(),
    }) {
        Ok(report) => format!("{} steps", report.plan.steps.len()),
        Err(error) => error.to_string(),
    };
}

/// Asserts the last `compile` step's answer.
#[then(expr = "compile answers {string}")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "cucumber parses a captured `{string}` into an owned value and passes it by value"
)]
fn answers(world: &mut Behavior, expected: String) {
    assert_eq!(world.answer, expected, "compile answered otherwise");
}
