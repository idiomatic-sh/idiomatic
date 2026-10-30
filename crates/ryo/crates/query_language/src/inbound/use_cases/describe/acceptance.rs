//! The steps of the `describe` feature: each a cucumber expression over the world, one to six lines, asserting the part's own results.

use cucumber::{given, then, when};

use crate::Behavior;
use std::sync::Arc;

use ryo_query_language::inbound::use_cases::describe::interface::DescribeUseCase as _;
use ryo_query_language::inbound::use_cases::describe::use_case::DescribeUseCaseImpl;

/// Names the scene the next `describe` step runs.
#[given(expr = "the describe input {string}")]
fn given_input(world: &mut Behavior, value: String) {
    world.input = value;
}

/// Runs `describe` over the scene the input names, keeping what it answered.
#[when(expr = "describe runs")]
fn runs(world: &mut Behavior) {
    let syntax = Arc::new(ryo_query_language::outbound::syntax::factory::winnow());
    let report = DescribeUseCaseImpl::new(syntax)
        .describe()
        .expect("describing cannot fail");
    world.answer = report
        .grammar
        .0
        .first()
        .map_or_else(String::new, |first| format!("starts at {}", first.name.0));
}

/// Asserts the last `describe` step's answer.
#[then(expr = "describe answers {string}")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "cucumber parses a captured `{string}` into an owned value and passes it by value"
)]
fn answers(world: &mut Behavior, expected: String) {
    assert_eq!(world.answer, expected, "describe answered otherwise");
}
