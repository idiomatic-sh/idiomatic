//! Native traversal projection is compared with the actual public owner output.
use super::super::super::fixtures;
use crate::application::file_dag::acceptance::operator_contract_test::fixtures::inputs;
use crate::domain::{
    operating::errors::OperatingError,
    operation_schema::types::InputShape,
    table_cells::types::{TreeForm, TreeRows},
    table_model::{service::kinds_of, types::Rows},
};
use ryo_language::domain::{elements::types::Elements, symbols::types::{ByteSlice, Symbols}};
use ryo_language::domain::builders::service::{enter_node, leave_node, table_builder, token};
use ryo_relations::domain::relations::{
    service::node_id,
    types::{CellKind, FileId, KindBits, Offset, Position, Relation, Value as Cell},
};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        plan_parts::types::{
            ColumnIndex, PlanComparison, Direction, Input as Source, JoinColumn, LiteralText, Operand,
            QueryLiteral, Side, StepId, Test,
        },
        plans::types::Operation,
        rows::{service::push_row, types::Table},
    },
};
#[derive(Clone, Copy)]
enum Scenario {
    NativeAxis,
    NativeJoinView,
}
type Input = Scenario;
type Value = usize;
type Failure = OperatingError;
const FROM: Source = Source::Relation(Relation::SyntaxTree);
fn column(index: u32) -> ColumnIndex {
    ColumnIndex(index)
}

fn axis() -> Operation {
    Operation::Axis {
        from: FROM,
        edges: FROM,
        domain: FROM,
        direction: Direction::Descendant,
        id: column(0),
        parent: column(0),
        child: column(1),
        domain_id: column(0),
        domain_filter: Vec::new(),
        domain_view: Vec::new(),
        output: Vec::new(),
    }
}
/// A real native tree with one child and preserved token text.
fn native_tree() -> (Elements, Symbols) {
    let mut symbols = Symbols::default();
    let mut elements = Elements::default();
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"x"));
    enter_node(&mut builder, KindBits(1), Offset(0));
    token(&mut builder, KindBits(2), Offset(0), Offset(1));
    leave_node(&mut builder, Offset(1));
    let mut finish = ryo_language::domain::completion::service::completion(
        &mut elements,
        &mut symbols,
        ByteSlice(b"x"),
        FileId(0),
    );
    ryo_language::domain::completion::service::name_kind(
        &mut finish,
        KindBits(1),
        ryo_relations::domain::terms::types::NameText("root"),
    );
    ryo_language::domain::completion::service::name_kind(
        &mut finish,
        KindBits(2),
        ryo_relations::domain::terms::types::NameText("token"),
    );
    ryo_language::domain::kind_index::service::index_kinds(&mut elements);
    (elements, symbols)
}

/// Representation-sensitive projection metadata used by native execution fixtures.
fn native_operation(scenario: Scenario) -> Operation {
    if matches!(scenario, Scenario::NativeAxis) {
        let mut operation = axis();
        if let Operation::Axis { output, .. } = &mut operation {
            *output = vec![
                JoinColumn {
                    side: Side::Right,
                    column: column(1),
                },
                JoinColumn {
                    side: Side::Left,
                    column: column(0),
                },
                JoinColumn {
                    side: Side::Right,
                    column: column(1),
                },
            ];
        }
        operation
    } else {
        Operation::Join {
            left: FROM,
            right: FROM,
            left_on: vec![column(0)],
            right_on: vec![column(1)],
            output: vec![
                JoinColumn {
                    side: Side::Right,
                    column: column(1),
                },
                JoinColumn {
                    side: Side::Left,
                    column: column(0),
                },
            ],
            filter: Vec::new(),
            view: vec![column(1), column(0)],

            reach: None,
        }
    }
}

fn native(scenario: Scenario) -> Result<Value, Failure> {
    let (elements, mut symbols) = native_tree();
    let mut starts = Table {
        kinds: vec![CellKind::Node],
        ..Table::default()
    };
    push_row(&mut starts, &[Cell::Node(node_id(FileId(0), Position(0)))]);
    let reached = Rows::Tree(TreeRows {
        elements: &elements,
        file: FileId(0),
        form: TreeForm::Elements,
    });
    let edges = Rows::Tree(TreeRows {
        elements: &elements,
        file: FileId(0),
        form: TreeForm::Edges,
    });
    let operation = native_operation(scenario);
    for empty in [false, true] {
        if empty {
            starts.cells.clear();
            starts.count.0 = 0;
        }
        let rows = if matches!(scenario, Scenario::NativeAxis) {
            vec![Rows::Computed(&starts), edges.clone(), reached.clone()]
        } else {
            vec![Rows::Computed(&starts), reached.clone()]
        };
        let schemas: Vec<_> = rows
            .iter()
            .map(|row| InputShape {
                kinds: kinds_of(row),
                tree: match row {
                    Rows::Tree(tree) => Some(tree.form),
                    _ => None,
                },
            })
            .collect();
        let answer =
            crate::domain::operation_schema::service::schema_of(&operation, &schemas, &[])?;
        let mut operation = operation.clone();
        let first = Operation::Select {
            from: Source::Relation(Relation::SyntaxTree),
            kept: vec![column(0)],
            comparisons: vec![PlanComparison {
                left: Operand::Column(column(1)),
                test: Test::Equal,
                right: Operand::Literal(QueryLiteral::Text(LiteralText(
                    if empty { "absent" } else { "root" }.into(),
                ))),
                origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
            }],
        };
        match &mut operation {
            Operation::Axis {
                from,
                edges,
                domain,
                ..
            } => {
                *from = Source::Step(StepId(0));
                *edges = Source::Relation(Relation::SyntaxChild);
                *domain = Source::Relation(Relation::SyntaxTree);
            }
            Operation::Join { left, right, .. } => {
                *left = Source::Step(StepId(0));
                *right = Source::Relation(Relation::SyntaxTree);
            }
            _ => panic!("the fixture lists every case it builds"),
        }
        let plan = inputs::plan(vec![first, operation]);
        let output = fixtures::requests::published_file(plan, vec![], vec![], b"x", &elements, &mut symbols)?;
        assert_eq!(answer, output.kinds);
        if !empty {
            assert!(output.count.0 > 0);
        }
    }
    Ok(if matches!(scenario, Scenario::NativeAxis) {
        3
    } else {
        2
    })
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("native_axis_projection_matches_execution"),
        word: Word::Succeeds,
        input: || Scenario::NativeAxis,
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("native_join_view_matches_execution"),
        word: Word::Succeeds,
        input: || Scenario::NativeJoinView,
        expect: Expect::Value(2),
    },
];
fn subject(scenario: Input) -> Result<Value, Failure> {
    native(scenario)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
