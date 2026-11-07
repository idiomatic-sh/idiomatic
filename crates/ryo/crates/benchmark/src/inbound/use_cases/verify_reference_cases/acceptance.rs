//! Actual public operation acceptance over independent source fixtures.
use cucumber::then;
use ryo_benchmark::inbound::use_cases::verify_reference_cases::fixtures::{
    Refusal, Scenario, exercise,
};
#[then("verify_reference_cases answers two independently authored cases")]
fn complete(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    assert_eq!(exercise(Scenario::Complete), Ok(2));
}
#[then("verify_reference_cases refuses missing input before a process launches")]
fn missing(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    assert_eq!(exercise(Scenario::Missing), Err(Refusal::MissingSource));
}
