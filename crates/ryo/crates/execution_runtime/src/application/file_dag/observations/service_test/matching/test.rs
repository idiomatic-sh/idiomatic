//! Request-qualified observation slots and the complete synthetic instruction census.
use super::fixtures::{CASES, subject};
#[cfg(test)]
use crate::cases::{Word, run};

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
