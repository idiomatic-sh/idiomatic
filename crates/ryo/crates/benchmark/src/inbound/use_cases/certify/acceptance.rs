//! Acceptance calls the public certification workflow over real immutable artifacts.
use cucumber::then;
use ryo_benchmark::domain::suite::types::CaseCount;
use ryo_benchmark::inbound::use_cases::certify::fixtures::{Change, exercise};
use ryo_benchmark::inbound::use_cases::certify::use_case::CertifyUseCaseImpl;
#[then("unchanged certification releases two verified cases")]
fn unchanged(world: &mut crate::Behavior) {
    let _ = world;
    assert_eq!(
        exercise(Change::None, CertifyUseCaseImpl::new),
        Ok(CaseCount(2))
    );
}
#[then("a changed patch refuses certified observation")]
fn changed(world: &mut crate::Behavior) {
    let _ = world;
    assert_eq!(exercise(Change::Patch, CertifyUseCaseImpl::new), Err(()));
}
