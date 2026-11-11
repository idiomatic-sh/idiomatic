//! The `join_result` cases: error translation tests the production contract without an execution fixture.
use super::fixtures::{CASES, subject};
use ryo_values::cases::{Word, run};

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
