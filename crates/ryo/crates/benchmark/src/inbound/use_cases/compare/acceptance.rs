//! Comparison acceptance proves multiset semantics through the public operation.
use cucumber::then;
use ryo_benchmark::domain::diagnostics::types::{BenchmarkFinding, RunReport};
use ryo_benchmark::inbound::use_cases::compare::{
    interface::CompareUseCase, use_case::CompareUseCaseImpl,
};
fn reports() -> (RunReport, RunReport) {
    let fixture = ryo_benchmark::outbound::worker::fixtures::data::report();
    let mut first = fixture.diagnostics[0].clone();
    first.start = ryo_benchmark::domain::diagnostics::types::SourceByteOffset(0);
    first.end = ryo_benchmark::domain::diagnostics::types::SourceByteOffset(1);
    let second = BenchmarkFinding {
        start: ryo_benchmark::domain::diagnostics::types::SourceByteOffset(2),
        end: ryo_benchmark::domain::diagnostics::types::SourceByteOffset(3),
        ..first.clone()
    };
    let reference = RunReport {
        files: fixture.files,
        diagnostics: vec![first, second],
    };
    let mut candidate = reference.clone();
    candidate.diagnostics.reverse();
    (reference, candidate)
}
#[then("equivalent diagnostic multisets agree")]
fn equivalent(world: &mut crate::Behavior) {
    let _ = world;
    let (reference, candidate) = reports();
    assert_eq!(
        CompareUseCaseImpl::new().compare(&reference, &candidate),
        Ok(())
    );
}
#[then("an extra diagnostic occurrence prevents agreement")]
fn duplicate(world: &mut crate::Behavior) {
    let _ = world;
    let (reference, mut candidate) = reports();
    candidate.diagnostics.push(candidate.diagnostics[0].clone());
    let difference = CompareUseCaseImpl::new()
        .compare(&reference, &candidate)
        .expect_err("duplicate occurrence");
    assert_eq!(
        (difference.reference.len(), difference.candidate.len()),
        (2, 3)
    );
}
