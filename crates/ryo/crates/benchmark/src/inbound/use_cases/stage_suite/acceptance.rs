//! Acceptance uses the same real public stage_suite operation.
use benchmark::inbound::use_cases::stage_suite::fixtures::{
    Refusal, Scenario, exercise, stage_missing,
};
use benchmark::inbound::use_cases::stage_suite::{
    interface::StageSuiteUseCase as _, use_case::StageSuiteUseCaseImpl,
};
use benchmark::outbound::suite::interface::SuitePort;
use benchmark_contracts::domain::reports::errors::ReportsError;
use cucumber::then;
use std::path::Path;

/// Stage a profile directory through the public use case.
fn stage(suite: &dyn SuitePort, directory: &Path, destination: &Path) -> Result<(), ReportsError> {
    StageSuiteUseCaseImpl::new(suite)
        .stage_suite(
            directory,
            destination,
            &[],
            benchmark_contracts::domain::workloads::types::StagePurpose::Certification,
        )
        .map(|_staged| ())
}

#[then("stage_suite subset preserves its promised result")]
fn subset(world: &mut crate::Behavior) -> Result<(), Box<dyn std::error::Error>> {
    let _: &mut crate::Behavior = world;
    exercise(Scenario::Subset)
}

#[then("stage_suite workload preserves its promised result")]
fn workload(world: &mut crate::Behavior) -> Result<(), Box<dyn std::error::Error>> {
    let _: &mut crate::Behavior = world;
    exercise(Scenario::Workload)
}

#[then("stage_suite supporting preserves its promised result")]
fn supporting(world: &mut crate::Behavior) -> Result<(), Box<dyn std::error::Error>> {
    let _: &mut crate::Behavior = world;
    exercise(Scenario::Supporting)
}

#[then("stage_suite discrepancy preserves its promised result")]
fn discrepancy(world: &mut crate::Behavior) -> Result<(), Box<dyn std::error::Error>> {
    let _: &mut crate::Behavior = world;
    exercise(Scenario::Discrepancy)
}

#[then("stage_suite staged profile preserves its promised result")]
fn staged_profile(world: &mut crate::Behavior) -> Result<(), Box<dyn std::error::Error>> {
    let _: &mut crate::Behavior = world;
    exercise(Scenario::StagedProfile)
}

#[then("stage_suite full profile preserves its promised result")]
fn full_profile(world: &mut crate::Behavior) -> Result<(), Box<dyn std::error::Error>> {
    let _: &mut crate::Behavior = world;
    exercise(Scenario::FullProfile)
}

#[then("stage_suite missing preserves its promised result")]
fn missing(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    assert_eq!(stage_missing(stage), Err(Refusal::MissingProfile));
}
