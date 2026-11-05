//! Acceptance uses the same real public stage_suite operation.
use cucumber::then;
use ryo_benchmark::inbound::use_cases::stage_suite::fixtures::{Refusal, Scenario, exercise};

#[then("stage_suite subset preserves its promised result")]
fn subset(_world: &mut crate::Behavior) {
    assert_eq!(exercise(Scenario::Subset), Ok(()));
}

#[then("stage_suite workload preserves its promised result")]
fn workload(_world: &mut crate::Behavior) {
    assert_eq!(exercise(Scenario::Workload), Ok(()));
}

#[then("stage_suite supporting preserves its promised result")]
fn supporting(_world: &mut crate::Behavior) {
    assert_eq!(exercise(Scenario::Supporting), Ok(()));
}

#[then("stage_suite discrepancy preserves its promised result")]
fn discrepancy(_world: &mut crate::Behavior) {
    assert_eq!(exercise(Scenario::Discrepancy), Ok(()));
}

#[then("stage_suite missing preserves its promised result")]
fn missing(_world: &mut crate::Behavior) {
    assert_eq!(exercise(Scenario::Missing), Err(Refusal::MissingProfile));
}
