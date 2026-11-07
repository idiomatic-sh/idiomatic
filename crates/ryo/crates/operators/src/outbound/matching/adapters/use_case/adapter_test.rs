//! The use-case adapter of the matching port: the port's contract rows run through it, then the cases specific to it.

use super::adapter::MatchingUseCaseAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::matching::fixtures::contract::{CASES as CONTRACT, answer};
use ryo_matching::inbound::use_cases::matching::errors::MatchingError;
use ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl;

type Input = (&'static str, &'static str, Option<&'static str>);
type Value = std::borrow::Cow<'static, [(usize, usize, Option<(usize, usize)>)]>;
type Failure = MatchingError;

/// The rows of what is specific to this adapter.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_zero_width_pattern_is_visited_at_every_boundary"),
    word: Word::Succeeds,
    input: || ("", "ab", None),
    expect: Expect::Value(std::borrow::Cow::Borrowed(&[(0, 0, None), (1, 1, None), (2, 2, None)])),
}];

/// Calls the adapter on one input, for the contract's rows and for its own.
fn subject(input: Input) -> Result<Value, Failure> {
    answer(
        &MatchingUseCaseAdapter::new(MatchingUseCaseImpl::new()),
        input,
    )
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
