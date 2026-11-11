//! Distinct state survives repeated left rows and resets after a failed delivery.
use super::super::fixtures::Scenario;
use super::fixtures::run as run_file;
use crate::cases::{Case, CaseName, Expect, Word, run};

fn reach_restarts_distinct_for_each_identical_left_row() {
    let actual = run_file(Scenario::ReachDuplicate).expect("compiled reach");
    assert_eq!(actual, vec![1, 2, 3, 1, 2, 3]);
}
fn project_and_union_retry_replay_completed_distinct_after_sink_failure() {
    for scenario in [Scenario::ProjectRetry, Scenario::UnionRetry] {
        let actual = run_file(scenario).expect("compiled retry");
        assert_eq!(actual, vec![0, 2]);
    }
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Proof {
    ReachRestartsDistinctForEachIdenticalLeftRow,
    ProjectAndUnionRetryReplayCompletedDistinctAfterSinkFailure,
}

type Input = Proof;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("reach_restarts_distinct_for_each_identical_left_row"),
        word: Word::Succeeds,
        input: || Proof::ReachRestartsDistinctForEachIdenticalLeftRow,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("project_and_union_retry_replay_completed_distinct_after_sink_failure"),
        word: Word::Succeeds,
        input: || Proof::ProjectAndUnionRetryReplayCompletedDistinctAfterSinkFailure,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Proof::ReachRestartsDistinctForEachIdenticalLeftRow => {
            reach_restarts_distinct_for_each_identical_left_row();
            Ok(())
        }
        Proof::ProjectAndUnionRetryReplayCompletedDistinctAfterSinkFailure => {
            project_and_union_retry_replay_completed_distinct_after_sink_failure();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
