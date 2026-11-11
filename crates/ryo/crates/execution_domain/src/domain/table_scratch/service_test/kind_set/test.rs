//! The `table_scratch` service's cases, on values alone.

use super::fixtures::{CASES, subject};

use ryo_values::cases::{Word, run};

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
