//! The prepare feature enters the real public preparation owner.
use crate::Behavior;
use cucumber::{given, then, when};
use ryo_execution::inbound::use_cases::prepare::interface::{ObservationIdentity, PrepareUseCase};
use ryo_execution::inbound::use_cases::prepare::types::{PlanSource, PrepareRequest};
use ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
use std::sync::Arc;

#[given(expr = "a prepare request {string}")]
fn request(world: &mut Behavior, input: String) {
    world.input = input;
}

#[when(expr = "preparation runs")]
fn runs(world: &mut Behavior) {
    let prepare = PrepareUseCaseImpl::new(
        Arc::new(
            ryo_execution::outbound::operators::factory::use_case(ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(Arc::new(
                ryo_operators::outbound::matching::factory::use_case(),
            ))),
        ),
        Arc::new(ryo_execution::outbound::derived::factory::semantics()),
        Arc::new(ryo_execution::outbound::digest::factory::blake3()),
    );
    let bindings = if world.input == "mismatched bindings" {
        vec![vec![]]
    } else {
        vec![]
    };
    let prepared = prepare.prepare(PrepareRequest {
        plans: PlanSource::Authored {
            plans: vec![],
            bindings,
        },
        rules: vec![],
        digests: vec![],
        hosted: vec![],
        observations: ObservationIdentity::Independent,
    });
    world.answer = match prepared {
        Ok(_) => "prepared",
        Err(_) => "refused",
    }
    .to_owned();
}

#[then(expr = "preparation answers {string}")]
fn answers(world: &mut Behavior, expected: String) {
    assert_eq!(world.answer, expected);
}
