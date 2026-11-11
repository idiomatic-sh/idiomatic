//! The support of the `selected_frontiers` cases, which the interpreter's twin cases also read.
use crate::boundary_fixture::fixtures::authored_request;
use ryo_language::domain::symbols::fixtures::intern_derived;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::fixtures::{kinds, node_id};
use ryo_relations::domain::relations::types::*;
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::plan_parts::types::*;
use ryo_values::domain::plans::types::*;
use ryo_values::domain::rows::types::{RowCount, Table};

pub(crate) fn selected_join(distinct: bool) -> Plan {
    let mut plan = crate::boundary_fixture::fixtures::plans(&mut authored_request(1)).remove(0);
    plan.steps = vec![
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: Input::Relation(Relation::Function),
                kept: vec![ColumnIndex(0), ColumnIndex(2)],
                comparisons: vec![PlanComparison {
                    test: Test::Equal,
                    left: Operand::Column(ColumnIndex(1)),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText("keep".into()))),
                    origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                }],
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Project {
                from: Input::Step(StepId(0)),
                columns: vec![ColumnIndex(u32::from(distinct))],
                distinct: if distinct {
                    RepeatedRows::Dropped
                } else {
                    RepeatedRows::Kept
                },
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Join {
                left: Input::Step(StepId(1)),
                right: Input::Step(StepId(0)),
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(u32::from(distinct))],
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
                filter: vec![],
                view: vec![],
                reach: None,
            },
        },
    ];
    plan.emit.step = StepId(2);
    plan.emit.subject = ColumnIndex(1);
    plan.frontier = vec![Input::Step(StepId(0))];
    plan
}

pub(crate) fn functions(symbols: &mut Symbols) -> Table {
    let keep = intern_derived(symbols, ByteSlice(b""), NameText("keep"));
    let skip = intern_derived(symbols, ByteSlice(b""), NameText("skip"));
    Table {
        kinds: kinds(Relation::Function).to_vec(),
        count: RowCount(64),
        cells: (0..64)
            .flat_map(|row| {
                [
                    Value::Node(node_id(FileId(0), Position(row))),
                    Value::Symbol(if row % 8 == 0 { keep } else { skip }),
                    Value::File(FileId(0)),
                    Value::Span(Span {
                        start: Offset(row),
                        end: Offset(row + 1),
                    }),
                ]
            })
            .collect(),
    }
}
