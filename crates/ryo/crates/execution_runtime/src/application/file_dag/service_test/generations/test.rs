//! Two file generation owners cannot reuse each other's results: each file's generation consults its own provider and keeps its output.
use super::fixtures::{CASES, subject};
use ryo_values::cases::{Word, run};

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
