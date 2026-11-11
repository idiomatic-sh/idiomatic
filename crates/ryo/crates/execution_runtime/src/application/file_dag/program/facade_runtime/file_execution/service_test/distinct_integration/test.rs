//! Distinct state survives repeated left rows and resets after a failed delivery.
use super::super::fixtures::Scenario;
use super::fixtures::run as run_file;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use std::borrow::Cow;

type Input = Scenario;
/// The node numbers the run delivered, in order.
type Value = Cow<'static, [u64]>;
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("reach_restarts_distinct_for_each_identical_left_row"),
        word: Word::Succeeds,
        input: || Scenario::ReachDuplicate,
        expect: Expect::Value(Cow::Borrowed(&[1, 2, 3, 1, 2, 3])),
    },
    Case {
        name: CaseName("project_and_union_retry_replay_completed_distinct_after_sink_failure"),
        word: Word::Succeeds,
        input: || Scenario::ProjectRetry,
        expect: Expect::Value(Cow::Borrowed(&[0, 2])),
    },
    Case {
        name: CaseName("union_retry_replays_completed_distinct_after_sink_failure"),
        word: Word::Succeeds,
        input: || Scenario::UnionRetry,
        expect: Expect::Value(Cow::Borrowed(&[0, 2])),
    },
];

/// Compiles the scenario's plan and runs it through one file.
fn subject(input: Input) -> Result<Value, Failure> {
    run_file(input).map(Cow::Owned)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
