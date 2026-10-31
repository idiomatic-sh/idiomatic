//! Measurement acceptance exercises the certified public operation.
use cucumber::then;
use ryo_benchmark::domain::suite::types::CaseCount;
use ryo_benchmark::inbound::use_cases::certify::fixtures::{Change, exercise};
use ryo_benchmark::inbound::use_cases::certify::use_case::CertifyUseCaseImpl;
#[then("matching reports release a measured pair")]
fn matching(world: &mut crate::Behavior) {
    let _ = world;
    assert_eq!(
        exercise(Change::None, CertifyUseCaseImpl::new),
        Ok(CaseCount(2))
    );
}
#[then("changed certified workload releases no measured pair")]
fn changed(world: &mut crate::Behavior) {
    let _ = world;
    assert_eq!(exercise(Change::Manifest, CertifyUseCaseImpl::new), Err(()));
}
