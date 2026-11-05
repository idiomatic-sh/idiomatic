//! Real public receipt admission, shared with its strict filesystem unit controls.
use cucumber::then;
use ryo_benchmark::inbound::use_cases::validate_case_receipts::fixtures::{Scenario, exercise};
#[then("receipt validation admits the shared producer registration")]
fn complete(_world: &mut crate::Behavior) {
    assert_eq!(exercise(Scenario::Complete), Ok(1));
}
#[then("receipt validation refuses a missing authority")]
fn authority(_world: &mut crate::Behavior) {
    assert_eq!(exercise(Scenario::Authority), Err("authority"));
}
#[then("receipt validation refuses a missing module inventory")]
fn modules(_world: &mut crate::Behavior) {
    assert_eq!(exercise(Scenario::Modules), Err("modules"));
}
#[then("receipt validation refuses a missing artifact inventory")]
fn artifacts(_world: &mut crate::Behavior) {
    assert_eq!(exercise(Scenario::Artifacts), Err("artifacts"));
}
