//! The omission falsifier runs the actual runtime and all other KernelSession calls.
use super::fixtures::{CASES, subject};
use crate::cases::{Word, run};
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn rejects() {
    run(CASES, Word::Rejects, subject);
}
