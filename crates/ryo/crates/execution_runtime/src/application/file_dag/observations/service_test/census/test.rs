//! The shared walk's census counts every synthetic observation union a compiled program adds, beside the original
//! instructions, each with its observation barrier.
use super::fixtures::{CASES, subject};
use crate::cases::{Word, run};

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
