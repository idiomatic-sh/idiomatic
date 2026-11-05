//! Real public receipt admission, shared with its strict filesystem unit controls.
use cucumber::then;
use ryo_benchmark::inbound::use_cases::validate_case_receipts::{
    fixtures::{Scenario, exercise},
    use_case::ValidateCaseReceiptsUseCaseImpl,
};
fn subject(scenario: Scenario) -> Result<usize, &'static str> {
    let use_case = ValidateCaseReceiptsUseCaseImpl::new(std::sync::Arc::new(
        ryo_benchmark::outbound::inventory::factory::local(),
    ));
    exercise(&use_case, scenario)
}
#[then("receipt validation admits the shared producer registration")]
fn complete(world: &mut crate::Behavior) {
    let _ = world;
    assert_eq!(subject(Scenario::Complete), Ok(1));
}
#[then("receipt validation refuses a missing authority")]
fn authority(world: &mut crate::Behavior) {
    let _ = world;
    assert_eq!(subject(Scenario::Authority), Err("authority"));
}
#[then("receipt validation refuses a missing module inventory")]
fn modules(world: &mut crate::Behavior) {
    let _ = world;
    assert_eq!(subject(Scenario::Modules), Err("modules"));
}
#[then("receipt validation refuses a missing artifact inventory")]
fn artifacts(world: &mut crate::Behavior) {
    let _ = world;
    assert_eq!(subject(Scenario::Artifacts), Err("artifacts"));
}
