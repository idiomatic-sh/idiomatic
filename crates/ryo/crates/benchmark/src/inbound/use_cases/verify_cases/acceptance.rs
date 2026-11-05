//! Actual public operation acceptance over independent source fixtures.
use cucumber::then;
use ryo_benchmark::inbound::use_cases::verify_cases::fixtures::{Refusal, Scenario, exercise};
#[then("verify_cases answers two independently authored cases")]
fn complete(_: &mut crate::Behavior) {
    assert_eq!(exercise(Scenario::Complete), Ok(2));
}
#[then("verify_cases refuses missing input before a process launches")]
fn missing(_: &mut crate::Behavior) {
    assert_eq!(exercise(Scenario::Missing), Err(Refusal::MissingSource));
}
