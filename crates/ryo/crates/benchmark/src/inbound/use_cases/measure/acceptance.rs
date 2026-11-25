//! Measurement acceptance exercises the certified public operation.
use benchmark::inbound::use_cases::certify::fixtures::{Change, exercise};
use benchmark::inbound::use_cases::certify::use_case::CertifyUseCaseImpl;
use benchmark_contracts::domain::workloads::types::case_count::value::CaseCount;
use cucumber::then;
/// Assert that matching reports release a measured pair.
#[then("matching reports release a measured pair")]
fn matching(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    assert_eq!(
        exercise(Change::None, CertifyUseCaseImpl::new),
        Ok(CaseCount(2)),
        "both independent cases must be verified"
    );
}
/// Assert that changed certified workload releases no measured pair.
#[then("changed certified workload releases no measured pair")]
fn changed(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    assert_eq!(
        exercise(Change::Manifest, CertifyUseCaseImpl::new),
        Err(()),
        "mutated certified artifacts must refuse measurement"
    );
}
