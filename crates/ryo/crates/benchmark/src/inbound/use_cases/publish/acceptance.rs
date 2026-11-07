//! Publication acceptance uses the actual certification and measurement pipeline.
use cucumber::then;
use ryo_benchmark::inbound::use_cases::certify::{
    fixtures::{Change, exercise},
    use_case::CertifyUseCaseImpl,
};
use ryo_benchmark_contracts::domain::workloads::types::case_count::value::CaseCount;
#[then("the complete certified inventory publishes its observed samples")]
fn complete(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    assert_eq!(
        exercise(Change::InventoryFull, CertifyUseCaseImpl::new),
        Ok(CaseCount(2))
    );
}
#[then("the diagnostic certified subset refuses metric publication")]
fn diagnostic(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    assert_eq!(
        exercise(Change::None, CertifyUseCaseImpl::new),
        Ok(CaseCount(2))
    );
}
