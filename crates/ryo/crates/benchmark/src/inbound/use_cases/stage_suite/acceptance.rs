//! Acceptance uses the same real public stage_suite operation.
use cucumber::then;
use ryo_benchmark::inbound::use_cases::stage_suite::fixtures::{
    Refusal, Scenario, exercise, stage_missing,
};

#[then("stage_suite subset preserves its promised result")]
fn subset(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    exercise(Scenario::Subset);
}

#[then("stage_suite workload preserves its promised result")]
fn workload(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    exercise(Scenario::Workload);
}

#[then("stage_suite supporting preserves its promised result")]
fn supporting(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    exercise(Scenario::Supporting);
}

#[then("stage_suite discrepancy preserves its promised result")]
fn discrepancy(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    exercise(Scenario::Discrepancy);
}

#[then("stage_suite missing preserves its promised result")]
fn missing(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    assert_eq!(stage_missing(), Err(Refusal::MissingProfile));
}
