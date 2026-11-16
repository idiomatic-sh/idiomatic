//! What every `host` implementation must pass.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::host::interface::HostPort;

type Input = ();
type Value = bool;
type Failure = std::convert::Infallible;

/// The contract's rows.
pub const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a host answers the same question the same way"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(true),
}];

/// Run the rows against one implementation.
///
/// # Panics
///
/// Panics when an implementation violates a contract row.
pub fn check(port: &impl HostPort) {
    run(CASES, Word::Succeeds, |()| Ok(port.facts() == port.facts()));
}
