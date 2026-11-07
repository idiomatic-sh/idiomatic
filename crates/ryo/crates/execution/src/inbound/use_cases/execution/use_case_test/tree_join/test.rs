//! A native tree on either side of an actual compiled Join publishes the same exact cells.
use super::super::super::fixtures;
use crate::application::file_dag::acceptance::operator_contract_test::fixtures::inputs;
use crate::inbound::use_cases::execution::interface::OperatingError;
use ryo_relations::domain::relations::{
    service::node_id,
    types::{FileId, Position, Relation, Value as Cell},
};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        plan_parts::types::{
            ColumnIndex, PlanComparison, Input as PlanInput, JoinColumn, LiteralText, Operand,
            QueryLiteral, Side, StepId, Test,
        },
        plans::types::Operation,
    },
};
type Input = ();
type Value = ();
type Failure = OperatingError;
fn subject((): Input) -> Result<Value, Failure> {
    let (elements, mut symbols) = fixtures::requests::tree_with_names();
    let has_text = PlanComparison {
        test: Test::NotEqual,
        left: Operand::Column(ColumnIndex(2)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText("".into()))),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    };
    let tokens = Operation::Select {
        from: PlanInput::Relation(Relation::SyntaxTree),
        comparisons: vec![has_text.clone()],
        kept: vec![ColumnIndex(0)],
    };
    let right = inputs::plan(vec![
        tokens.clone(),
        Operation::Join {
            left: PlanInput::Step(StepId(0)),
            right: PlanInput::Relation(Relation::SyntaxTree),
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
            filter: vec![has_text],
            view: vec![ColumnIndex(0), ColumnIndex(2)],
            output: vec![
                JoinColumn {
                    side: Side::Left,
                    column: ColumnIndex(0),
                },
                JoinColumn {
                    side: Side::Right,
                    column: ColumnIndex(1),
                },
            ],
            reach: None,
        },
    ]);
    let left = inputs::plan(vec![
        tokens,
        Operation::Join {
            left: PlanInput::Relation(Relation::SyntaxTree),
            right: PlanInput::Step(StepId(0)),
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
            filter: vec![],
            view: vec![],
            output: vec![
                JoinColumn {
                    side: Side::Left,
                    column: ColumnIndex(0),
                },
                JoinColumn {
                    side: Side::Left,
                    column: ColumnIndex(2),
                },
            ],
            reach: None,
        },
    ]);
    let right = fixtures::requests::published_file(right, vec![], vec![], b"abcd", &elements, &mut symbols)?;
    let left = fixtures::requests::published_file(left, vec![], vec![], b"abcd", &elements, &mut symbols)?;
    assert_eq!(right.count.0, 2);
    assert_eq!(left.count.0, 2);
    assert_eq!(right.cells, left.cells);
    assert_eq!(right.cells[0], Cell::Node(node_id(FileId(0), Position(1))));
    assert_eq!(right.cells[2], Cell::Node(node_id(FileId(0), Position(3))));
    Ok(())
}
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_tree_joins_as_many_rows_on_either_side"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(()),
}];
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
