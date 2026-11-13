//! Consuming the configured handle forwards the same request and exact pair accounting.
use super::value::ConfiguredEngineSession;
use crate::{
    cases::{Case, CaseName, Expect, Word, run},
    outbound::{
        engine::{fixtures::fake::EngineFake, interface::EnginePort, types::EngineConfiguration},
        worker::fixtures::data::request,
    },
};
type Input = ();
type Value = usize;
type Failure = std::convert::Infallible;
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("configured execution preserves the exact loaded request and accounting"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(1),
}];
fn subject((): Input) -> Result<Value, Failure> {
    let request = request();
    let engine = EngineFake::default();
    let loaded = engine.load(&request).expect("valid loaded request");
    let configured = ConfiguredEngineSession::new(
        loaded,
        EngineConfiguration {
            selected_missing: Default::default(),
            selected_rules: match request.selected_rules.clone() {
                ryo_benchmark_contracts::domain::requests::types::CohortSelection::Explicit(
                    selected,
                ) => selected.into_iter().collect(),
                ryo_benchmark_contracts::domain::requests::types::CohortSelection::Frozen => {
                    panic!("selected rules")
                }
            },
            selected_pairs: crate::outbound::engine::types::SelectedPairs(1),
            hosted: Vec::new(),
        },
    );
    let report = configured
        .execute()
        .expect("actual port fake executes")
        .report;
    assert_eq!(report.files, request.files);
    let ryo_benchmark_contracts::domain::reports::types::CoverageAccounting::Accounted(coverage) =
        report.coverage
    else {
        panic!("accounting retained");
    };
    Ok(coverage.selected.0)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
