//! Every transport preserves the canonical request and accepts a canonical report.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::worker::interface::WorkerPort;

/// Request and response observations shared across implementations.
#[derive(Clone, Copy, Debug)]
pub enum Operation {
    /// Read the canonical request from the injected transport.
    Receive,
    /// Publish the canonical report through the injected transport.
    Respond,
}
type Input = Operation;
type Value = bool;
type Failure = String;
/// Canonical transport observations; adapter-specific tests inspect actual response bytes.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("request fields survive transport"),
        word: Word::Succeeds,
        input: || Operation::Receive,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("canonical report can be published"),
        word: Word::Succeeds,
        input: || Operation::Respond,
        expect: Expect::Value(true),
    },
];
use super::data::{report, request};
/// Execute the transport contract against an implementation configured with `request()`.
/// # Panics
/// Panics if a contract observation does not match its expected value.
pub fn check(adapter: &impl WorkerPort) {
    let expected = request();
    let subject = |operation| match operation {
        Operation::Receive => adapter
            .receive()
            .map(|actual| {
                actual.root == expected.root
                    && actual.rules == expected.rules
                    && actual.files == expected.files
                    && actual.state == expected.state
            })
            .map_err(|error| error.to_string()),
        Operation::Respond => adapter
            .respond(&report())
            .map(|()| true)
            .map_err(|error| error.to_string()),
    };
    run(CASES, Word::Succeeds, subject);
}
