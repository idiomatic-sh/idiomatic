//! The rules compile ahead of the run through the session the run is configured with.
use super::super::{Engine, Stage};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::inbound::use_cases::run_ryo::{
    fixtures::CountingContext, interface::RunRyoUseCase as _, use_case::RunRyoUseCaseImpl,
};
use crate::outbound::worker::fixtures::{data::request, fake::WorkerFake};
use std::sync::{Arc, atomic::AtomicUsize};
use std::time::Duration;

/// The request the worker receives.
type Input = ();
/// How many rules report a time, and whether every one reports a nonzero time.
type Value = (usize, bool);
type Failure = String;

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("every rule of the configured session reports a nonzero compile time"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value((1, true)),
}];

fn subject(_request: Input) -> Result<Value, Failure> {
    let engine = Arc::new(Engine {
        calls: AtomicUsize::new(0),
        loaded: AtomicUsize::new(0),
        closed: AtomicUsize::new(0),
        stage: Stage::Complete,
    });
    let compiled = RunRyoUseCaseImpl::new(
        engine,
        Arc::new(WorkerFake::new(request())),
        Arc::new(CountingContext::default()),
    )
    .compile()
    .ok_or_else(|| "the session compiles nothing".to_owned())?;
    Ok((
        compiled.len(),
        compiled.values().all(|elapsed| *elapsed > Duration::ZERO),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
