//! A native tree on either side of an actual compiled Join publishes the same exact cells.
use super::super::super::fixtures;
use crate::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::inputs;
use ryo_relations::domain::relations::service::node_position;
use ryo_relations::domain::relations::types::{Relation, Value as Cell};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        plan_parts::types::{
            ColumnIndex, Input as PlanInput, JoinColumn, LiteralText, Operand, PlanComparison,
            QueryLiteral, Side, StepId, Test,
        },
        plans::types::Operation,
    },
};
use std::borrow::Cow;
/// What the tree on the right and the tree on the left published.
#[derive(Debug, PartialEq, Eq)]
struct Trees {
    /// Row counts, right then left.
    rows: (usize, usize),
    /// Both sides published the same exact cells.
    same_cells: bool,
    /// The positions of the right side's node cells, in order.
    nodes: Cow<'static, [u32]>,
}
type Input = ();
type Value = Trees;
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
    let right = fixtures::requests::published_file(
        right,
        vec![],
        vec![],
        b"abcd",
        &elements,
        &mut symbols,
    )?;
    let left =
        fixtures::requests::published_file(left, vec![], vec![], b"abcd", &elements, &mut symbols)?;
    Ok(Trees {
        rows: (right.count.0, left.count.0),
        same_cells: right.cells == left.cells,
        nodes: right
            .cells
            .iter()
            .filter_map(|cell| {
                if let Cell::Node(node) = cell {
                    Some(node_position(*node).0)
                } else {
                    None
                }
            })
            .collect(),
    })
}
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_tree_joins_as_many_rows_on_either_side"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(Trees {
        rows: (2, 2),
        same_cells: true,
        nodes: Cow::Borrowed(&[1, 3]),
    }),
}];
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
