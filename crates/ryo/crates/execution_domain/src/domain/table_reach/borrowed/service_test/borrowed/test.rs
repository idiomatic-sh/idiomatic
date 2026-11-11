//! Differential traversal behavior over an actual prepared equality index.
use super::fixtures::{CASES, subject};

use ryo_values::cases::{Word, run};

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
