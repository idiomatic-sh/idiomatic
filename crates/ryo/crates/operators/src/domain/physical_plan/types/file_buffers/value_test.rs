//! Buffers answer no rows for a node they do not hold, and keep answering none once it is released.
use super::value::FileBuffers;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::physical_plan::types::{NodeRows, PlanNodeId};

#[derive(Debug, PartialEq)]
enum Held {
    Absent,
    Materialized,
}

/// Whether the node is released before it is asked for.
type Input = Released;
type Value = Held;
type Failure = ();

#[derive(Clone, Copy)]
enum Released {
    Never,
    Before,
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_node_never_run_has_no_rows"),
        word: Word::Succeeds,
        input: || Released::Never,
        expect: Expect::Value(Held::Absent),
    },
    Case {
        name: CaseName("a_released_node_has_no_rows"),
        word: Word::Succeeds,
        input: || Released::Before,
        expect: Expect::Value(Held::Absent),
    },
];

fn subject(released: Input) -> Result<Value, Failure> {
    let mut buffers = FileBuffers::default();
    if matches!(released, Released::Before) {
        buffers.release(PlanNodeId(0));
    }
    Ok(match buffers.rows(PlanNodeId(0)) {
        NodeRows::Materialized(_) => Held::Materialized,
        NodeRows::Absent => Held::Absent,
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
