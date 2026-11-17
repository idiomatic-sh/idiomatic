//! The closure through `plan` and `run` only, keyed by a partition: each reached vertex one row after its key and source,
//! each partition under its own key, and no row from no edge, as main's row-mode cases stated; and a seeded closure, fused
//! into its join, answering every row past a chunk's cells.
use super::super::super::fixtures::edges::{EDGES, edge_rows};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::fixtures::Step;
use crate::domain::physical_plan::types::PlanInput;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input as StepInput, JoinColumn, ReachLookup, Side,
};
use ryo_values::domain::plans::types::Operation;

#[derive(Clone, Copy)]
enum Scenario {
    ClosureOfOnePartition,
    ClosureOfTwoPartitions,
    ClosureOfNoEdge,
    SeededOfManyAnswers,
}
#[derive(Debug, PartialEq)]
enum Observation {
    /// The rows answered, each cell a node's position or a number.
    Rows(std::borrow::Cow<'static, [i64]>),
    /// How many rows were answered.
    Count(usize),
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
    Case {
        name: CaseName("a_seeded_closure_answers_every_row_past_a_chunk"),
        word: Word::Succeeds,
        input: || Scenario::SeededOfManyAnswers,
        // 20 chains of 40 edges: the parent at place `k` of its chain reaches the `40 - k` nodes after it.
        expect: Expect::Value(Observation::Count(16_400)),
    },
];

/// A join of the edges, as left rows, with their closure on the left's parent, the closure fused into it (seeded from the
/// left's parents): the left's parent and each node it reaches.
fn seeded_join() -> Step {
    let side = |side, column| JoinColumn {
        side,
        column: ColumnIndex(column),
    };
    Step {
        operation: Operation::Join {
            left: StepInput::Relation(EDGES),
            right: StepInput::Relation(EDGES),
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
            output: vec![side(Side::Left, 0), side(Side::Right, 1)],
            filter: Vec::new(),
            view: Vec::new(),
            reach: Some(ReachLookup {
                source: ColumnIndex(0),
                destination: ColumnIndex(1),
                keys: Vec::new(),
                columns: vec![ColumnIndex(0), ColumnIndex(1)],
            }),
            fan: ryo_values::domain::plan_parts::types::KeyFan::Shared,
        },
        inputs: vec![PlanInput::Relation(EDGES), PlanInput::Relation(EDGES)],
    }
}

/// 20 chains of 40 edges each, its edges' answers past a chunk's 16 Ki cells.
fn chains() -> Vec<(u32, u32, i64)> {
    (0..20_u32)
        .flat_map(|chain| (chain * 41..chain * 41 + 40).map(|at| (at, at + 1, 0)))
        .collect()
}

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
        // Each answer is 2 cells: the parent and a node it reaches.
        Scenario::SeededOfManyAnswers => {
            return Ok(Observation::Count(
                edge_rows(seeded_join(), &chains())?.len() / 2,
            ));
        }
    }?;
    Ok(Observation::Rows(rows.into()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
