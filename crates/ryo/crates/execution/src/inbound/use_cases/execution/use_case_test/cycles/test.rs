//! Cyclic nodes are published by real hosted-edge execution, including full and empty graphs.
use super::super::super::fixtures;
use crate::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::inputs;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{Integer, Relation, Value as Cell};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        plan_parts::types::{ColumnIndex, HostedNumber, Input as PlanInput, StepId},
        plans::types::Operation,
    },
};
use std::borrow::Cow;
/// The edge rows the cycles operation reads, and whether they reach it through its real hosted
/// provider or it reads the function relation instead.
#[derive(Clone, Copy, Debug)]
struct Edges {
    rows: &'static [&'static [i64]],
    hosted: bool,
}

type Input = Edges;
/// The rows published: each node on a cycle, at both endpoints.
type Value = Cow<'static, [[Cell; 2]]>;
type Failure = OperatingError;

/// A cycle output row of `node` at both endpoints.
const fn on_cycle(node: i64) -> [Cell; 2] {
    [Cell::Number(Integer(node)), Cell::Number(Integer(node))]
}

/// A path from 1 through 2 to 3, and from 1 to 3.
const PATH: &[&[i64]] = &[&[1, 2], &[2, 3], &[1, 3]];

/// Publishes the cycles over the edges.
fn subject(edges: Input) -> Result<Value, Failure> {
    let mut operations = vec![];
    let hosted = if edges.hosted {
        operations.push(Operation::Hosted {
            relation: HostedNumber(0),
        });
        vec![fixtures::requests::numbers("edges", 2, edges.rows)]
    } else {
        vec![]
    };
    operations.push(Operation::Cycles {
        edges: if edges.hosted {
            PlanInput::Step(StepId(0))
        } else {
            PlanInput::Relation(Relation::Function)
        },
        source: ColumnIndex(0),
        destination: ColumnIndex(1),
    });
    let output = fixtures::requests::published(
        inputs::plan(operations),
        vec![],
        hosted,
        &mut Symbols::default(),
    )?;
    Ok(Cow::Owned(
        output
            .cells
            .chunks_exact(2)
            .map(|row| [row[0].clone(), row[1].clone()])
            .collect(),
    ))
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_row_for_each_node_that_is_on_a_cycle"),
        word: Word::Succeeds,
        input: || Edges {
            rows: &[&[1, 2], &[2, 1], &[2, 3]],
            hosted: true,
        },
        expect: Expect::Value(Cow::Borrowed(&[on_cycle(1), on_cycle(2)])),
    },
    Case {
        name: CaseName("a_ring_and_a_self_loop_are_cycles_and_a_node_only_reached_is_not"),
        word: Word::Succeeds,
        input: || Edges {
            rows: &[
                &[1, 2],
                &[2, 1],
                &[2, 7],
                &[3, 4],
                &[4, 5],
                &[5, 3],
                &[6, 6],
            ],
            hosted: true,
        },
        expect: Expect::Value(Cow::Borrowed(&[
            on_cycle(1),
            on_cycle(2),
            on_cycle(3),
            on_cycle(4),
            on_cycle(5),
            on_cycle(6),
        ])),
    },
    Case {
        name: CaseName("a_path_that_does_not_return_is_no_cycle"),
        word: Word::Succeeds,
        input: || Edges {
            rows: PATH,
            hosted: true,
        },
        expect: Expect::Value(Cow::Borrowed(&[])),
    },
    Case {
        name: CaseName("cycles_require_their_real_provider"),
        word: Word::Refuses,
        input: || Edges {
            rows: PATH,
            hosted: false,
        },
        expect: Expect::Error(OperatingError::NotRead),
    },
];
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
