//! The identity check receives through the transport and refuses a request that is not the executable's.
use super::{
    interface::CheckWorkerRequestUseCase as _, types::WorkerExpectation,
    use_case::CheckWorkerRequestUseCaseImpl,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::worker::fixtures::{data::request, fake::WorkerFake};
use ryo_benchmark_contracts::domain::requests::types::{
    ApplicabilityPolicy, ParserConfigurationName, ParserSelection, RuleLabel,
};
use std::{collections::BTreeMap, sync::Arc};

type Input = &'static str;
type Value = bool;
type Failure = &'static str;

const TARGET: &str = "python-target=3.10";
const OTHER_TARGET: &str = "python-target=3.11";
const RULE: &str = "EXE004";

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a request carrying the expected target is accepted"),
        word: Word::Succeeds,
        input: || TARGET,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a request retargeted to another version is refused"),
        word: Word::Refuses,
        input: || OTHER_TARGET,
        expect: Expect::Error("worker parser configuration differs from its profile identity"),
    },
];

fn subject(received: Input) -> Result<Value, Failure> {
    let mut worker_request = request();
    worker_request.parser_configuration =
        ParserSelection::Named(ParserConfigurationName(received.into()));
    worker_request.rule_applicability =
        BTreeMap::from([(RuleLabel(RULE.into()), ApplicabilityPolicy("always".into()))]);
    let expectation = WorkerExpectation {
        parser_configuration: ParserConfigurationName(TARGET.into()),
        pinned_applicability: BTreeMap::new(),
    };
    CheckWorkerRequestUseCaseImpl::new(Arc::new(WorkerFake::new(worker_request)))
        .check_worker_request(&expectation)
        .map(|()| true)
        .map_err(|error| &*Box::leak(error.to_string().into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
