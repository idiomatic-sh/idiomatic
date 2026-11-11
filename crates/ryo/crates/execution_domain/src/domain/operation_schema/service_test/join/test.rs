//! Missing RHS is a schema/input refusal, independent of any owner or source.
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
