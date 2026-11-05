//! Acceptance uses the same real public prepare_cases operation.
use cucumber::then;
use ryo_benchmark::inbound::use_cases::prepare_cases::fixtures::{Refusal, Scenario, exercise};

#[then("prepare_cases complete preserves its promised result")]
fn complete(_world: &mut crate::Behavior) {
    assert_eq!(exercise(Scenario::Complete), Ok(2));
}

#[then("prepare_cases missing preserves its promised result")]
fn missing(_world: &mut crate::Behavior) {
    assert_eq!(exercise(Scenario::Missing), Err(Refusal::MissingSource));
}
