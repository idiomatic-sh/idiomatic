//! Producer declarations and actual source scope govern ordinary admission.
use super::super::*;
use super::fixtures::CASES;
use super::fixtures::subject;
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
