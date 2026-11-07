//! The steps of the `list_operators` feature: each a cucumber expression over the world, one to six lines, asserting the part's own results.

use cucumber::{given, then, when};

use crate::Behavior;
use ryo_core::inbound::use_cases::list_operators::interface::ListOperatorsUseCase as _;
use ryo_core::inbound::use_cases::list_operators::use_case::ListOperatorsUseCaseImpl;

/// Names the scene the next `list_operators` step runs.
#[given(expr = "the list_operators input {string}")]
fn given_input(world: &mut Behavior, value: String) {
    world.input = value;
}

/// Runs `list_operators` over the scene the input names, keeping what it answered.
#[when(expr = "list_operators runs")]
fn runs(world: &mut Behavior) {
    let report = ListOperatorsUseCaseImpl::new(
        ryo_core::inbound::use_cases::lint::fixtures::engine::metadata(),
    )
    .list_operators()
        .expect("listing cannot fail");
    world.answer = format!("{} operators", report.operators.len());
}

/// Asserts what the last `list_operators` step answered.
#[then(expr = "list_operators answers {string}")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "cucumber parses a step's `{string}` into an owned `String` and passes it by value"
)]
fn answers(world: &mut Behavior, expected: String) {
    assert_eq!(world.answer, expected, "list_operators answered otherwise");
}
