//! Demand compilation preserves declared order independently of physical delivery.
use super::{service::demand, types::DemandAction};
use crate::domain::{
    operation_schema::types::InputShape, operator::contract::service::prepare_for,
    operator::contract::types::DemandOrder,
};
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        plan_parts::types::{ColumnIndex, Direction, Input as PlanInput},
        plans::types::{Operation, OperationKind},
    },
};
type Input = (DemandOrder, usize);
type Value = Vec<usize>;
type Failure = crate::domain::operating::errors::OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("no_inputs"),
        word: Word::Succeeds,
        input: || (DemandOrder::None, 0),
        expect: Expect::Value(vec![]),
    },
    Case {
        name: CaseName("unary"),
        word: Word::Succeeds,
        input: || (DemandOrder::Unary, 1),
        expect: Expect::Value(vec![0]),
    },
    Case {
        name: CaseName("empty_ordered"),
        word: Word::Succeeds,
        input: || (DemandOrder::LeftThenRight, 0),
        expect: Expect::Value(vec![]),
    },
    Case {
        name: CaseName("one_ordered"),
        word: Word::Succeeds,
        input: || (DemandOrder::LeftThenRight, 1),
        expect: Expect::Value(vec![0]),
    },
    Case {
        name: CaseName("two_ordered"),
        word: Word::Succeeds,
        input: || (DemandOrder::LeftThenRight, 2),
        expect: Expect::Value(vec![0, 1]),
    },
    Case {
        name: CaseName("three_ordered"),
        word: Word::Succeeds,
        input: || (DemandOrder::LeftThenRight, 3),
        expect: Expect::Value(vec![0, 1, 2]),
    },
];
fn subject((order, count): Input) -> Result<Value, Failure> {
    let operation = Operation::Axis {
        from: PlanInput::Relation(Relation::Function),
        edges: PlanInput::Relation(Relation::SyntaxChild),
        domain: PlanInput::Relation(Relation::SyntaxTree),
        direction: Direction::Child,
        id: ColumnIndex(0),
        parent: ColumnIndex(0),
        child: ColumnIndex(1),
        domain_id: ColumnIndex(0),
        domain_filter: vec![],
        domain_view: vec![],
        output: vec![],
    };
    let inputs = [
        InputShape {
            kinds: &[CellKind::Node],
            tree: None,
        },
        InputShape {
            kinds: &[CellKind::Node, CellKind::Node],
            tree: None,
        },
        InputShape {
            kinds: &[CellKind::Node],
            tree: None,
        },
    ];
    let mut contract = prepare_for(OperationKind::Axis, &operation, &inputs, &[])?;
    assert_eq!(contract.semantics.demand, DemandOrder::LeftThenRight);
    contract.semantics.demand = order;
    let program = demand(contract.semantics, count);
    Ok(program
        .actions()
        .iter()
        .map(|action| match action {
            DemandAction::Admit(input) => input.0,
        })
        .collect())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
