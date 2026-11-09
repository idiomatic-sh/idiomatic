//! Acceptance uses the same real public prepare_cases operation.
use cucumber::then;
use ryo_benchmark::inbound::use_cases::prepare_cases::fixtures::{Refusal, Scenario, exercise};
use ryo_benchmark::inbound::use_cases::prepare_cases::{
    interface::PrepareCasesUseCase as _, use_case::PrepareCasesUseCaseImpl,
};
use ryo_benchmark::outbound::suite::interface::SuitePort;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use std::path::Path;

/// Prepare the authored cases through the public use case.
fn prepare(suite: &dyn SuitePort, rules: &Path, scratch: &Path) -> Result<usize, ReportsError> {
    PrepareCasesUseCaseImpl::new(suite, &ryo_benchmark::outbound::context::factory::local())
        .prepare_cases(rules, scratch)
        .map(|cases| {
            assert_eq!(
                cases
                    .iter()
                    .map(|case| case.expected.diagnostics.len())
                    .collect::<Vec<_>>(),
                [1, 0]
            );
            cases.len()
        })
}

#[then("prepare_cases complete preserves its promised result")]
fn complete(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    assert_eq!(exercise(Scenario::Complete, prepare), Ok(2));
}

#[then("prepare_cases missing preserves its promised result")]
fn missing(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    assert_eq!(
        exercise(Scenario::Missing, prepare),
        Err(Refusal::MissingSource)
    );
}
