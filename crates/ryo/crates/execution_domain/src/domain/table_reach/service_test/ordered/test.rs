//! Boundary values for the actual borrowed graph preparation and traversal APIs.
use super::fixtures::{CASES, subject};
use ryo_values::cases::{Word, run};

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
