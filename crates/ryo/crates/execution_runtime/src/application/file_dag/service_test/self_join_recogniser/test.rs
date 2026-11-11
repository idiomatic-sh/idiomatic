//! The `self_join_recogniser` service's cases for a request of two rules, through the compiled request.

use super::fixtures::{CASES, subject};
use ryo_values::cases::{Word, run};

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
