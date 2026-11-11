//! Ordered graph traversals that refuse name the exact error.
use super::super::fixtures::subject;
use super::fixtures::CASES;
use ryo_values::cases::{Word, run};
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
