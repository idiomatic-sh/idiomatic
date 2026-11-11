//! Ordered graph traversals that succeed keep exact order, multiplicity and work counts.
use super::super::fixtures::subject;
use super::fixtures::CASES;
use ryo_values::cases::{Word, run};
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
