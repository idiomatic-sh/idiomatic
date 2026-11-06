//! Native raw acquisition survives zero retention and preserves first faults through cleanup.
use super::fixtures::{RawCase, observe};
use crate::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::operating::errors::OperatingError,
};
use ryo_values::domain::rows::types::RowCount;
type Input = RawCase;
type Value = RowCount;
type Failure = OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("native_raw_uses_required_owner_without_optional_retention"),
        word: Word::Succeeds,
        input: || RawCase::Complete,
        expect: Expect::Value(RowCount(3)),
    },
    Case {
        name: CaseName("native_raw_streaming_releases_shared_ledger"),
        word: Word::Succeeds,
        input: || RawCase::Streaming,
        expect: Expect::Value(RowCount(3)),
    },
    Case {
        name: CaseName("raw_port_failure_is_not_rewritten"),
        word: Word::Fails,
        input: || RawCase::DerivedFailure,
        expect: Expect::Error(OperatingError::DuplicateOrder),
    },
    Case {
        name: CaseName("callback_failure_releases_raw_lease"),
        word: Word::Fails,
        input: || RawCase::SemanticFailure,
        expect: Expect::Error(OperatingError::Table(
            ryo_values::domain::rows::errors::RowsError::NoSuchColumn { index: 60001 },
        )),
    },
    Case {
        name: CaseName("sink_failure_releases_raw_scope_and_publications"),
        word: Word::Fails,
        input: || RawCase::SinkFailure,
        expect: Expect::Error(OperatingError::NotRun),
    },
];
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}

fn subject(input: Input) -> Result<Value, Failure> {
    observe(input)
}
