//! Acceptance calls the public certification workflow over real immutable artifacts.
use cucumber::then;
use ryo_benchmark::inbound::use_cases::certify::fixtures::{Change, exercise};
use ryo_benchmark::inbound::use_cases::certify::use_case::CertifyUseCaseImpl;
use ryo_benchmark_contracts::domain::workloads::types::case_count::value::CaseCount;
/// Assert that unchanged certification releases two verified cases.
#[then("unchanged certification releases two verified cases")]
fn unchanged(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    assert_eq!(
        exercise(Change::None, CertifyUseCaseImpl::new),
        Ok(CaseCount(2)),
        "both independent cases must be verified"
    );
}
/// Assert that a changed patch refuses certified observation.
#[then("a changed patch refuses certified observation")]
fn changed(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    assert_eq!(
        exercise(Change::Patch, CertifyUseCaseImpl::new),
        Err(()),
        "mutated certified artifacts must refuse measurement"
    );
}
