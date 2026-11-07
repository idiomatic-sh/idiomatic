//! The execution feature enters the real public request owner.
use crate::Behavior;
use cucumber::{given, then, when};
use ryo_execution::inbound::use_cases::execution::{
    interface::{ExecutionRequest, ExecutionUseCase},
    types::{ObservationIdentity, PreparedRequest},
    use_case::ExecutionUseCaseImpl,
};
use std::sync::Arc;

#[given(expr = "an execution request {string}")]
fn request(world: &mut Behavior, input: String) {
    world.input = input;
}

#[when(expr = "execution runs")]
fn runs(world: &mut Behavior) {
    let engine = ExecutionUseCaseImpl::new(
        Arc::new(
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case())),
        ),
        Arc::new(crate::outbound::derived::factory::semantics()),
    );
    let bindings = if world.input == "mismatched bindings" {
        vec![vec![]]
    } else {
        vec![]
    };
    let prepared = engine.prepare(PreparedRequest {
        plans: vec![],
        bindings,
        rules: vec![],
        digests: vec![],
        hosted: vec![],
        observations: ObservationIdentity::Independent,
    });
    world.answer = match prepared {
        Ok(prepared) => {
            engine.begin(&prepared, &[]).finish();
            "complete"
        }
        Err(_) => "refused",
    }
    .to_owned();
}

#[then(expr = "execution answers {string}")]
fn answers(world: &mut Behavior, expected: String) {
    assert_eq!(world.answer, expected);
}
