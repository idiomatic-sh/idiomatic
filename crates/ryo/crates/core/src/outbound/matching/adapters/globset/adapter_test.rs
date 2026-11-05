//! The globset adapter of the matching port: the port's contract rows run through it, then the cases specific to it.

use super::adapter::MatchingGlobsetAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::matching::fixtures::contract::{CASES as CONTRACT, answer};

type Input = (&'static [&'static str], &'static str);
type Value = usize;
type Failure = ryo_execution::inbound::use_cases::execution::types::matching::MatchingError;

/// The rows of what is specific to this adapter.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_glob_for_a_root_file_matches_it_without_a_directory"),
    word: Word::Succeeds,
    input: || (&["**/Cargo.lock"], "Cargo.lock"),
    expect: Expect::Value(1),
}];

/// Calls the adapter on one input, for the contract's rows and for its own.
fn subject((globs, path): Input) -> Result<Value, Failure> {
    answer(&MatchingGlobsetAdapter, globs, path)
}

#[test]
fn succeeds() {
    run(CONTRACT, Word::Succeeds, subject);
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CONTRACT, Word::Refuses, subject);
}
