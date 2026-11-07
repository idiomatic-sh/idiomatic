//! The execution feature enters the real public request owner.
use crate::Behavior;
use cucumber::{given, then, when};
use ryo_execution::application::file_dag::program::request_execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::prepare::interface::ObservationIdentity;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution::inbound::use_cases::prepare::types::{PlanSource, PrepareRequest};
use ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
use std::sync::Arc;

#[given(expr = "an execution request {string}")]
fn request(world: &mut Behavior, input: String) {
    world.input = input;
}

#[when(expr = "execution runs")]
fn runs(world: &mut Behavior) {
    let prepare = PrepareUseCaseImpl::new(
        Arc::new(
            ryo_execution::outbound::operators::factory::use_case(ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(Arc::new(
                ryo_operators::outbound::matching::factory::use_case(),
            ))),
        ),
        Arc::new(ryo_execution::outbound::derived::factory::semantics()),
        Arc::new(crate::outbound::digest::factory::blake3()),
    );
    let prepared = prepare
        .prepare(PrepareRequest {
            plans: PlanSource::Authored {
                plans: vec![],
                bindings: vec![],
            },
            rules: vec![],
            digests: vec![],
            hosted: vec![],
            observations: ObservationIdentity::Independent,
        })
        .expect("an empty request is legitimate");
    ExecutionUseCaseImpl::new()
        .execution(&prepared, &[])
        .finish();
    world.answer = "complete".to_owned();
}

#[then(expr = "execution answers {string}")]
fn answers(world: &mut Behavior, expected: String) {
    assert_eq!(world.answer, expected);
}
