//! Demand compilation consumes actual prepared Axis publication policy for every supply tuple.
use super::{
    service::demand,
    types::{DemandAction, HeldJoinOrder},
};
use crate::domain::{
    operating::types::Supply,
    operation_schema::types::InputShape,
    operator::contract::{service::prepare_for, types::Publication},
};
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        plan_parts::types::{ColumnIndex, Direction, Input as PlanInput},
        plans::types::{Operation, OperationKind},
    },
};
type Input = [Supply; 3];
type Value = Publication;
type Failure = crate::domain::operating::errors::OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("athand_athand_athand"),
        word: Word::Succeeds,
        input: || [Supply::AtHand, Supply::AtHand, Supply::AtHand],
        expect: Expect::Value(Publication::AfterSuccess),
    },
    Case {
        name: CaseName("athand_athand_streamed"),
        word: Word::Succeeds,
        input: || [Supply::AtHand, Supply::AtHand, Supply::Streamed],
        expect: Expect::Value(Publication::AfterSuccess),
    },
    Case {
        name: CaseName("athand_streamed_athand"),
        word: Word::Succeeds,
        input: || [Supply::AtHand, Supply::Streamed, Supply::AtHand],
        expect: Expect::Value(Publication::AfterSuccess),
    },
    Case {
        name: CaseName("athand_streamed_streamed"),
        word: Word::Succeeds,
        input: || [Supply::AtHand, Supply::Streamed, Supply::Streamed],
        expect: Expect::Value(Publication::AfterSuccess),
    },
    Case {
        name: CaseName("streamed_athand_athand"),
        word: Word::Succeeds,
        input: || [Supply::Streamed, Supply::AtHand, Supply::AtHand],
        expect: Expect::Value(Publication::Incremental),
    },
    Case {
        name: CaseName("streamed_athand_streamed"),
        word: Word::Succeeds,
        input: || [Supply::Streamed, Supply::AtHand, Supply::Streamed],
        expect: Expect::Value(Publication::AfterSuccess),
    },
    Case {
        name: CaseName("streamed_streamed_athand"),
        word: Word::Succeeds,
        input: || [Supply::Streamed, Supply::Streamed, Supply::AtHand],
        expect: Expect::Value(Publication::AfterSuccess),
    },
    Case {
        name: CaseName("streamed_streamed_streamed"),
        word: Word::Succeeds,
        input: || [Supply::Streamed, Supply::Streamed, Supply::Streamed],
        expect: Expect::Value(Publication::AfterSuccess),
    },
];
fn subject(supplies: Input) -> Result<Value, Failure> {
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
    let contract = prepare_for(OperationKind::Axis, &operation, &inputs, &[])?;
    let program = demand(
        contract.streaming,
        3,
        HeldJoinOrder::LeftWithEmptyGate,
        &supplies,
    );
    Ok(
        if program
            .actions()
            .contains(&DemandAction::PublishAfterSuccess)
        {
            Publication::AfterSuccess
        } else {
            Publication::Incremental
        },
    )
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
