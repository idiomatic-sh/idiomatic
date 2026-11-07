//! Actual public operation acceptance over independent source fixtures.
use cucumber::then;
use ryo_benchmark::inbound::use_cases::verify_reference_cases::fixtures::{
    Refusal, Scenario, exercise,
};
use ryo_benchmark::inbound::use_cases::verify_reference_cases::{
    interface::VerifyReferenceCasesUseCase as _, use_case::VerifyReferenceCasesUseCaseImpl,
};
use ryo_benchmark::outbound::{
    execution::interface::ExecutionPort, linter::interface::LinterPort,
    suite::interface::SuitePort,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use std::path::Path;

/// Verify the authored reference cases through the public use case.
fn verify(
    suite: &dyn SuitePort,
    reference: &dyn LinterPort,
    execution: &dyn ExecutionPort,
    rules: &Path,
    scratch: &Path,
) -> Result<usize, ReportsError> {
    let inventory = ryo_benchmark::outbound::inventory::factory::local();
    let context = ryo_benchmark::outbound::context::factory::local();
    let receipts = ryo_benchmark::outbound::receipts::fixtures::fake::ReceiptsFake::default();
    VerifyReferenceCasesUseCaseImpl::new(suite, &receipts, &context, reference, execution, &inventory)
        .verify_reference_cases(rules, scratch)
        .map(|count| count.0)
}

#[then("verify_reference_cases answers two independently authored cases")]
fn complete(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    assert_eq!(exercise(Scenario::Complete, verify), Ok(2));
}
#[then("verify_reference_cases refuses missing input before a process launches")]
fn missing(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    assert_eq!(exercise(Scenario::Missing, verify), Err(Refusal::MissingSource));
}
