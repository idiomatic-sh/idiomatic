//! Comparison acceptance proves multiset semantics through the public operation.
#![expect(
    clippy::expect_used,
    reason = "acceptance assertions must fail the scenario on unexpected success or refusal"
)]
use cucumber::then;
use ryo_benchmark::domain::diagnostics::types::{BenchmarkFinding, RunReport};
use ryo_benchmark::inbound::use_cases::compare::{
    interface::CompareUseCase as _, use_case::CompareUseCaseImpl,
};
/// Build the same two findings in opposite traversal orders.
fn reports() -> (RunReport, RunReport) {
    let fixture = ryo_benchmark::outbound::worker::fixtures::data::report();
    let mut first = fixture
        .diagnostics
        .first()
        .expect("fixture finding")
        .clone();
    first.start = ryo_benchmark::domain::diagnostics::types::SourceByteOffset(0);
    first.end = ryo_benchmark::domain::diagnostics::types::SourceByteOffset(1);
    let second = BenchmarkFinding {
        start: ryo_benchmark::domain::diagnostics::types::SourceByteOffset(2),
        end: ryo_benchmark::domain::diagnostics::types::SourceByteOffset(3),
        ..first.clone()
    };
    let reference = RunReport {
        execution_metadata: ryo_benchmark::domain::diagnostics::types::ExecutionMetadata::Complete,
        project_context: None,
        files: fixture.files,
        skipped: Vec::new(),
        coverage: Some(ryo_benchmark::domain::diagnostics::types::RuleCoverage {
            selected: 1,
            evaluated: 1,
            skipped: 0,
            unknown_or_unread: 0,
        }),
        diagnostics: vec![first, second],
    };
    let mut candidate = reference.clone();
    candidate.diagnostics.reverse();
    (reference, candidate)
}
/// Assert that equivalent diagnostic multisets agree.
#[then("equivalent diagnostic multisets agree")]
fn equivalent(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    let (reference, candidate) = reports();
    assert_eq!(
        CompareUseCaseImpl::new().compare(&reference, &candidate),
        Ok(()),
        "diagnostic order must not affect equality"
    );
}
/// Assert that an extra diagnostic occurrence prevents agreement.
#[then("an extra diagnostic occurrence prevents agreement")]
fn duplicate(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    let (reference, mut candidate) = reports();
    candidate.diagnostics.push(
        candidate
            .diagnostics
            .first()
            .expect("candidate finding")
            .clone(),
    );
    let difference = CompareUseCaseImpl::new()
        .compare(&reference, &candidate)
        .expect_err("duplicate occurrence");
    assert_eq!(
        (difference.reference.len(), difference.candidate.len()),
        (2, 3),
        "duplicate occurrence must survive normalization"
    );
}
