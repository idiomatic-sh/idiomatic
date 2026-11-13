//! Distinct state survives repeated left rows and resets after a failed delivery.
use super::super::fixtures::PlanShape;
use super::fixtures::{Delivered, Retry, run as run_file};
use crate::application::file_dag::program::runtime::types::{
    AttemptCheckpoint, DependencyCheckpoint,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use std::borrow::Cow;

type Input = PlanShape;
type Value = Delivered;
type Failure = OperatingError;

/// Nodes 0 and 2, delivered by the retry after the sink refused the first delivery: the
/// refusal published nothing, kept the completed publication, and the retry replayed it.
const fn retried() -> Delivered {
    Delivered {
        nodes: Cow::Borrowed(&[0, 2]),
        retry: Some(Retry {
            before: (AttemptCheckpoint::Dormant, DependencyCheckpoint::Pending),
            refusal: Some(OperatingError::WrongBinding),
            refused_rows: 0,
            refused: (
                AttemptCheckpoint::Published,
                DependencyCheckpoint::Consumed,
                true,
            ),
            retried_published: true,
            complete: (AttemptCheckpoint::Published, DependencyCheckpoint::Consumed),
        }),
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("reach_restarts_distinct_for_each_identical_left_row"),
        word: Word::Succeeds,
        input: || PlanShape::ReachDuplicate,
        expect: Expect::Value(Delivered {
            nodes: Cow::Borrowed(&[1, 2, 3, 1, 2, 3]),
            retry: None,
        }),
    },
    Case {
        name: CaseName("project_and_union_retry_replay_completed_distinct_after_sink_failure"),
        word: Word::Succeeds,
        input: || PlanShape::ProjectRetry,
        expect: Expect::Value(retried()),
    },
    Case {
        name: CaseName("union_retry_replays_completed_distinct_after_sink_failure"),
        word: Word::Succeeds,
        input: || PlanShape::UnionRetry,
        expect: Expect::Value(retried()),
    },
];

/// Compiles the scenario's plan and runs it through one file.
fn subject(input: Input) -> Result<Value, Failure> {
    run_file(input)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
