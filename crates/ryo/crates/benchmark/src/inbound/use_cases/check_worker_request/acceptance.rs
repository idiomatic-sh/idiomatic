//! The public worker identity check over the transport double.
use cucumber::then;
use ryo_benchmark::inbound::use_cases::check_worker_request::{
    interface::CheckWorkerRequestUseCase as _, types::WorkerExpectation,
    use_case::CheckWorkerRequestUseCaseImpl,
};
use ryo_benchmark::outbound::worker::fixtures::{data::request, fake::WorkerFake};
use ryo_benchmark_contracts::domain::requests::types::{
    ApplicabilityPolicy, ParserConfigurationName, ParserSelection, RuleLabel,
};
use std::{collections::BTreeMap, sync::Arc};

fn check(received: &str) -> bool {
    let mut worker_request = request();
    worker_request.parser_configuration =
        ParserSelection::Named(ParserConfigurationName(received.into()));
    worker_request.rule_applicability = BTreeMap::from([(
        RuleLabel("EXE004".into()),
        ApplicabilityPolicy("always".into()),
    )]);
    let expectation = WorkerExpectation {
        parser_configuration: ParserConfigurationName("python-target=3.10".into()),
        pinned_applicability: BTreeMap::new(),
    };
    CheckWorkerRequestUseCaseImpl::new(Arc::new(WorkerFake::new(worker_request)))
        .check_worker_request(&expectation)
        .is_ok()
}

#[then("check_worker_request accepts the request carrying the expected identity")]
fn accepts(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    assert!(check("python-target=3.10"));
}

#[then("check_worker_request refuses a request retargeted to another version")]
fn refuses(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    assert!(!check("python-target=3.11"));
}
