//! Actual public operation acceptance over independent source fixtures.
use benchmark::inbound::use_cases::verify_cases::fixtures::{Refusal, Scenario, exercise};
use benchmark::inbound::use_cases::verify_cases::{
    interface::VerifyCasesUseCase as _, use_case::VerifyCasesUseCaseImpl,
};
use benchmark::outbound::{
    execution::interface::ExecutionPort, linter::interface::LinterPort, suite::interface::SuitePort,
};
use benchmark_contracts::domain::reports::errors::ReportsError;
use cucumber::then;
use std::path::Path;

/// Verify the authored cases through the public use case.
fn verify(
    suite: &dyn SuitePort,
    reference: &dyn LinterPort,
    candidate: &dyn LinterPort,
    execution: &dyn ExecutionPort,
    rules: &Path,
    scratch: &Path,
) -> Result<usize, ReportsError> {
    let inventory = benchmark::outbound::inventory::factory::local();
    let context = benchmark::outbound::context::factory::local();
    let receipts = benchmark::outbound::receipts::fixtures::fake::ReceiptsFake::default();
    VerifyCasesUseCaseImpl::new(
        suite,
        &benchmark::outbound::host::fixtures::fake::HostFake::default(),
        &receipts,
        &context,
        reference,
        candidate,
        benchmark::inbound::use_cases::verify_cases::types::Executions {
            reference: execution,
            candidate: execution,
        },
        &inventory,
        &benchmark::outbound::evidence::fixtures::fake::EvidenceFake,
    )
    .verify_cases(
        rules,
        scratch,
        benchmark_contracts::domain::requests::types::CaseSelection::default(),
        benchmark_contracts::domain::requests::types::BenchmarkCoverage::Accounted,
    )
    .map(|count| count.0)
}

#[then("verify_cases answers two independently authored cases")]
fn complete(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    assert_eq!(exercise(Scenario::Complete, verify), Ok(2));
}
#[then("verify_cases refuses missing input before a process launches")]
fn missing(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    assert_eq!(
        exercise(Scenario::Missing, verify),
        Err(Refusal::MissingSource)
    );
}
