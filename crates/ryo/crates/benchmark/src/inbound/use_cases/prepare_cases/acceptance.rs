//! Acceptance uses the same real public prepare_cases operation.
use cucumber::then;
use ryo_benchmark::inbound::use_cases::prepare_cases::fixtures::{Refusal, Scenario, exercise};

#[then("prepare_cases complete preserves its promised result")]
fn complete(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    assert_eq!(exercise(Scenario::Complete), Ok(2));
}

#[then("prepare_cases missing preserves its promised result")]
fn missing(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    assert_eq!(exercise(Scenario::Missing), Err(Refusal::MissingSource));
}
