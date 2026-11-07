//! The `memory_diagnostics` service's cases: without a subscriber of their target the diagnostics are off and walk nothing.
use super::service::memory_diagnostics;
use super::types::MemoryDiagnostics;
use crate::cases::{Case, CaseName, Expect, Word, run};

/// What a row asks of the diagnostics.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// Whether they are on.
    Gate,
}

type Input = Asked;
type Value = MemoryDiagnostics;
type Failure = std::convert::Infallible;

fn subject(asked: Input) -> Result<Value, Failure> {
    Ok(match asked {
        Asked::Gate => memory_diagnostics(),
    })
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("without_a_subscriber_the_diagnostics_are_off"),
        word: Word::Succeeds,
        input: || Asked::Gate,
        expect: Expect::Value(MemoryDiagnostics::Disabled),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
