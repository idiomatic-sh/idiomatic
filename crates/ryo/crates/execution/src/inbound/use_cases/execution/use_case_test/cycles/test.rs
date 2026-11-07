//! Cyclic nodes are published by real hosted-edge execution, including full and empty graphs.
use super::super::super::fixtures;
use crate::application::file_dag::acceptance::operator_contract_test::fixtures::inputs;
use crate::inbound::use_cases::execution::interface::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{Integer, Relation, Value as Cell};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        plan_parts::types::{ColumnIndex, HostedNumber, Input as PlanInput, StepId},
        plans::types::Operation,
    },
};
#[derive(Clone, Copy)]
enum Scenario {
    OneCycle,
    Rings,
    Path,
    Missing,
}
type Input = Scenario;
type Value = usize;
type Failure = OperatingError;
fn subject(input: Input) -> Result<Value, Failure> {
    let rows: &[&[i64]] = match input {
        Scenario::OneCycle => &[&[1, 2], &[2, 1], &[2, 3]],
        Scenario::Rings => &[
            &[1, 2],
            &[2, 1],
            &[2, 7],
            &[3, 4],
            &[4, 5],
            &[5, 3],
            &[6, 6],
        ],
        _ => &[&[1, 2], &[2, 3], &[1, 3]],
    };
    let mut operations = vec![];
    let hosted = if matches!(input, Scenario::Missing) {
        vec![]
    } else {
        operations.push(Operation::Hosted {
            relation: HostedNumber(0),
        });
        vec![fixtures::numbers("edges", 2, rows)]
    };
    operations.push(Operation::Cycles {
        edges: if matches!(input, Scenario::Missing) {
            PlanInput::Relation(Relation::Function)
        } else {
            PlanInput::Step(StepId(0))
        },
        source: ColumnIndex(0),
        destination: ColumnIndex(1),
    });
    let output = fixtures::published(
        inputs::plan(operations),
        vec![],
        hosted,
        &mut Symbols::default(),
    )?;
    let expected: &[i64] = match input {
        Scenario::OneCycle => &[1, 2],
        Scenario::Rings => &[1, 2, 3, 4, 5, 6],
        _ => &[],
    };
    let actual: Vec<_> = output
        .cells
        .chunks_exact(2)
        .map(|row| {
            assert_eq!(
                row[0], row[1],
                "cycle output retains the same node at both endpoints"
            );
            let Cell::Number(Integer(node)) = row[0] else {
                panic!("numeric endpoint required")
            };
            node
        })
        .collect();
    assert_eq!(actual, expected);
    Ok(output.count.0)
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_row_for_each_node_that_is_on_a_cycle"),
        word: Word::Succeeds,
        input: || Scenario::OneCycle,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a_ring_and_a_self_loop_are_cycles_and_a_node_only_reached_is_not"),
        word: Word::Succeeds,
        input: || Scenario::Rings,
        expect: Expect::Value(6),
    },
    Case {
        name: CaseName("a_path_that_does_not_return_is_no_cycle"),
        word: Word::Succeeds,
        input: || Scenario::Path,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("cycles_require_their_real_provider"),
        word: Word::Refuses,
        input: || Scenario::Missing,
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
