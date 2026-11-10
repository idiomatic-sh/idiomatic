//! The `metadata` service's cases: the decisions over request metadata it hands a use case know the operators.

use super::service::kernel_metadata;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution::inbound::use_cases::metadata::interface::MetadataUseCase as _;

/// What a row of the cases asks of the service.
type Input = ();
/// Whether the service answered as the row expects.
type Value = bool;
/// What a row of the cases expects when it is refused.
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("the_decisions_it_hands_out_know_the_operators"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(true),
}];

/// Asks the service for its decisions and for the operators they know.
fn subject(_: Input) -> Result<Value, Failure> {
    Ok(!kernel_metadata().operators().is_empty())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
