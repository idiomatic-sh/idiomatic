//! A forwarding port preserves the issued handle's exact request and loaded map.
use super::value::LoadedEngineSession;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::{
    engine::{fixtures::fake::EngineFake, interface::EnginePort},
    worker::fixtures::data::request,
};
use ryo_benchmark_contracts::domain::{
    reports::errors::ReportsError, requests::types::WorkerRequest,
};
type Input = ();
type Value = usize;
type Failure = std::convert::Infallible;
struct Forward<T>(T);
impl<T: EnginePort> EnginePort for Forward<T> {
    fn load<'a>(
        &'a self,
        request: &'a WorkerRequest,
    ) -> Result<LoadedEngineSession<'a>, ReportsError> {
        self.0.load(request)
    }
}
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("external-style forwarding preserves the actual loaded association"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(2),
}];
fn subject((): Input) -> Result<Value, Failure> {
    let request = request();
    let forward = Forward(EngineFake::default());
    let mut loaded = forward.load(&request).expect("valid fake request");
    let bound = loaded.request();
    let rules = loaded.rules_mut();
    assert_eq!(bound.files, request.files);
    Ok(usize::from(std::ptr::eq(bound, &request)) + rules.len())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
