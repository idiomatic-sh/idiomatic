//! The steps of the `describe_operator` feature: each a cucumber expression over the world, one to six lines, asserting the part's own results.

use cucumber::{given, then, when};

use crate::Behavior;
use ryo_core::inbound::use_cases::describe_operator::interface::DescribeOperatorUseCase as _;
use ryo_core::inbound::use_cases::describe_operator::types::DescribeOperatorRequest;
use ryo_core::inbound::use_cases::describe_operator::use_case::DescribeOperatorUseCaseImpl;
use ryo_values::domain::descriptions::types::OperatorName;

/// Names the scene the next `describe_operator` step runs.
#[given(expr = "the describe_operator input {string}")]
fn given_input(world: &mut Behavior, value: String) {
    world.input = value;
}

/// Runs `describe_operator` over the scene the input names, keeping what it answered.
#[when(expr = "describe_operator runs")]
fn runs(world: &mut Behavior) {
    let request = DescribeOperatorRequest {
        name: OperatorName(world.input.as_str().into()),
    };
    world.answer = match DescribeOperatorUseCaseImpl::new(
        ryo_core::inbound::use_cases::lint::fixtures::engine::metadata(),
    )
    .describe_operator(request) {
        Ok(report) => format!("{} fields", report.operator.fields.len()),
        Err(error) => error.to_string(),
    };
}

/// Asserts what the last `describe_operator` step answered.
#[then(expr = "describe_operator answers {string}")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "cucumber parses a step's `{string}` into an owned `String` and passes it by value"
)]
fn answers(world: &mut Behavior, expected: String) {
    assert_eq!(
        world.answer, expected,
        "describe_operator answered otherwise"
    );
}
