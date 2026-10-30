//! The steps of the `describe_relation` feature: each a cucumber expression over the world, one to six lines, asserting the part's own results.

use cucumber::{given, then, when};

use crate::Behavior;
use std::sync::Arc;

use ryo_core::inbound::use_cases::describe_relation::interface::DescribeRelationUseCase as _;
use ryo_core::inbound::use_cases::describe_relation::types::DescribeRelationRequest;
use ryo_core::inbound::use_cases::describe_relation::use_case::DescribeRelationUseCaseImpl;
use ryo_core::outbound::language::fixtures::fake::LanguageFake;
use ryo_values::domain::descriptions::types::RelationName;

/// Names the scene the next `describe_relation` step runs.
#[given(expr = "the describe_relation input {string}")]
fn given_input(world: &mut Behavior, value: String) {
    world.input = value;
}

/// Runs `describe_relation` over the scene the input names, keeping what it answered.
#[when(expr = "describe_relation runs")]
fn runs(world: &mut Behavior) {
    let use_case = DescribeRelationUseCaseImpl::new(Arc::new(LanguageFake::default()));
    world.answer = match use_case.describe_relation(DescribeRelationRequest {
        name: RelationName(world.input.as_str().into()),
    }) {
        Ok(report) => format!("{} columns", report.relation.columns.len()),
        Err(error) => error.to_string(),
    };
}

/// Asserts what the last `describe_relation` step answered.
#[then(expr = "describe_relation answers {string}")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "cucumber parses a step's `{string}` into an owned `String` and passes it by value"
)]
fn answers(world: &mut Behavior, expected: String) {
    assert_eq!(
        world.answer, expected,
        "describe_relation answered otherwise"
    );
}
