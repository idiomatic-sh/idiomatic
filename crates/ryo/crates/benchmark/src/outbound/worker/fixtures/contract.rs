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
    /// Encode the canonical report without publishing it.
    Encode,
    /// Read the canonical request stored in a file.
    Read,
}
/// Transport operation to exercise.
type Input = Operation;
/// Whether canonical request or response values survive transport.
type Value = bool;
/// Human-readable transport failure.
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
    Case {
        name: CaseName("canonical report encodes to bytes without being published"),
        word: Word::Succeeds,
        input: || Operation::Encode,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a stored request reads back with its fields"),
        word: Word::Succeeds,
        input: || Operation::Read,
        expect: Expect::Value(true),
    },
];
use super::data::{report, request};
/// Execute the transport contract against an implementation configured with `request()`, which `stored` also holds.
/// # Panics
/// Panics if a contract observation does not match its expected value.
pub fn check(adapter: &impl WorkerPort, stored: &std::path::Path) {
    let expected = request();
    let same = |actual: benchmark_contracts::domain::requests::types::WorkerRequest| {
        actual.root == expected.root
            && actual.rules == expected.rules
            && actual.files == expected.files
            && actual.state == expected.state
    };
    let subject = |operation| match operation {
        Operation::Receive => adapter
            .receive()
            .map(same)
            .map_err(|error| error.to_string()),
        Operation::Read => adapter
            .read(stored)
            .map(same)
            .map_err(|error| error.to_string()),
        Operation::Respond => adapter
            .respond(&report())
            .map(|()| true)
            .map_err(|error| error.to_string()),
        Operation::Encode => adapter
            .encoded(&report())
            .map(|bytes| !bytes.0.is_empty())
            .map_err(|error| error.to_string()),
    };
    run(CASES, Word::Succeeds, subject);
}
