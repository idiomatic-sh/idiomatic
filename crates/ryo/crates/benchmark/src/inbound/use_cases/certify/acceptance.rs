//! Acceptance calls the public certification workflow over real immutable artifacts.
use cucumber::then;
use ryo_benchmark::domain::suite::types::CaseCount;
use ryo_benchmark::inbound::use_cases::certify::fixtures::{Change, exercise};
use ryo_benchmark::inbound::use_cases::certify::use_case::CertifyUseCaseImpl;
/// Assert that unchanged certification releases two verified cases.
#[then("unchanged certification releases two verified cases")]
fn unchanged(_world: &mut crate::Behavior) {
    assert_eq!(
        exercise(Change::None, CertifyUseCaseImpl::new),
        Ok(CaseCount(2)),
        "both independent cases must be verified"
    );
}
/// Assert that a changed patch refuses certified observation.
#[then("a changed patch refuses certified observation")]
fn changed(_world: &mut crate::Behavior) {
    assert_eq!(
        exercise(Change::Patch, CertifyUseCaseImpl::new),
        Err(()),
        "mutated certified artifacts must refuse measurement"
    );
}
