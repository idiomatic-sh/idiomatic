//! The closure through `plan` and `run` only, keyed by a partition: each reached vertex one row after its key and source,
//! each partition under its own key, and no row from no edge, as main's row-mode cases stated.
use super::super::super::fixtures::edges::{EDGES, edge_rows};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::fixtures::Step;
use crate::domain::physical_plan::types::PlanInput;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as StepInput};
use ryo_values::domain::plans::types::Operation;

#[derive(Clone, Copy)]
enum Scenario {
    ClosureOfOnePartition,
    ClosureOfTwoPartitions,
    ClosureOfNoEdge,
}
#[derive(Debug, PartialEq)]
enum Observation {
    /// The rows answered, each cell a node's position or a number.
    Rows(std::borrow::Cow<'static, [i64]>),
}
type Input = Scenario;
type Value = Observation;
type Failure = PortableOperatorFault;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_reached_vertex_is_one_row_after_its_key_and_source"),
        word: Word::Succeeds,
        input: || Scenario::ClosureOfOnePartition,
        // A closure keyed on the ordinal: key 10, source 1, each entity it reaches (3, then 4).
        expect: Expect::Value(Observation::Rows(std::borrow::Cow::Borrowed(&[
            10, 1, 3, 10, 1, 4,
        ]))),
    },
    Case {
        name: CaseName("each_partition_answers_under_its_own_key"),
        word: Word::Succeeds,
        input: || Scenario::ClosureOfTwoPartitions,
        expect: Expect::Value(Observation::Rows(std::borrow::Cow::Borrowed(&[
            10, 1, 3, 11, 2, 5,
        ]))),
    },
    Case {
        name: CaseName("no_partition_answers_no_row"),
        word: Word::Succeeds,
        input: || Scenario::ClosureOfNoEdge,
        expect: Expect::Value(Observation::Rows(std::borrow::Cow::Borrowed(&[]))),
    },
];

/// A closure of the edges keyed on their ordinal, from each parent to each child it reaches.
fn keyed_closure() -> Step {
    Step {
        operation: Operation::Closure {
            edges: StepInput::Relation(EDGES),
            keys: vec![ColumnIndex(2)],
            source: ColumnIndex(0),
            destination: ColumnIndex(1),
        },
        inputs: vec![PlanInput::Relation(EDGES)],
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let rows = match scenario {
        Scenario::ClosureOfOnePartition => edge_rows(keyed_closure(), &[(1, 3, 10), (1, 4, 10)]),
        Scenario::ClosureOfTwoPartitions => edge_rows(keyed_closure(), &[(1, 3, 10), (2, 5, 11)]),
        Scenario::ClosureOfNoEdge => edge_rows(keyed_closure(), &[]),
    }?;
    Ok(Observation::Rows(rows.into()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
