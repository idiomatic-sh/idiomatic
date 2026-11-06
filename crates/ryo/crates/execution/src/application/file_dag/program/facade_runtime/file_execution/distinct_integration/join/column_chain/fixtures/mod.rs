//! The column-chain plan, stored relations and sources the join proofs share.
use super::super::*;
use ryo_language::domain::symbols::service::intern_derived;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::plan_parts::types::{
    Comparison, Input, LiteralText, Operand, QueryLiteral, Test,
};
pub(crate) fn column(number: u32) -> ColumnIndex {
    ColumnIndex(number)
}
/// The same compiled family observed at PYI034 cut 116 and PLC0206 cut 233.
pub(in super::super) fn plan() -> Plan {
    let steps = vec![
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Project {
                from: Input::Relation(Relation::Function),
                columns: vec![column(0), column(2)],
                distinct: RepeatedRows::Dropped,
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Join {
                left: Input::Step(StepId(0)),
                right: Input::Relation(Relation::SemanticChild),
                left_on: vec![column(0), column(1)],
                right_on: vec![column(0), column(4)],
                output: vec![
                    JoinColumn {
                        side: Side::Left,
                        column: column(0),
                    },
                    JoinColumn {
                        side: Side::Left,
                        column: column(1),
                    },
                    JoinColumn {
                        side: Side::Right,
                        column: column(1),
                    },
                    JoinColumn {
                        side: Side::Right,
                        column: column(3),
                    },
                ],
                filter: Vec::new(),
                view: Vec::new(),
                reach: None,
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: Input::Step(StepId(1)),
                comparisons: vec![Comparison {
                    test: Test::Equal,
                    left: Operand::Column(column(3)),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText("body".into()))),
                    origin: None,
                }],
                kept: Vec::new(),
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Project {
                from: Input::Step(StepId(2)),
                columns: vec![column(0), column(1), column(2)],
                distinct: RepeatedRows::Dropped,
            },
        },
    ];
    Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
        parameters: Vec::new(),
        steps,
        comparison_origins: Vec::new(),
        emit: Emit {
            step: StepId(3),
            subject: column(0),
            key: vec![column(0)],
            headings: Vec::new(),
        },
        // The shared cut is also handed outward, exactly as a real kept cut is.
        frontier: vec![Input::Step(StepId(0))],
    }
}
pub(in super::super) fn stored(relation: Relation, table: Table) -> Provider<'static> {
    let columns = (0..table.kinds.len())
        .map(|column| {
            FactColumn(
                table
                    .cells
                    .chunks_exact(table.kinds.len())
                    .map(|row| to_bits(row[column]).0)
                    .collect(),
            )
        })
        .collect();
    Provider {
        relation,
        table,
        columns,
        compact: None,
        tree: None,
        representation: RepresentationKind::Stored,
    }
}
pub(in super::super) fn sources(
    symbols: &mut Symbols,
    count: u32,
    hit: bool,
) -> [Provider<'static>; 2] {
    let body = intern_derived(symbols, ByteSlice(b""), NameText("body"));
    let other = intern_derived(symbols, ByteSlice(b""), NameText("other"));
    let file = FileId(0);
    let left = Table {
        kinds: kinds(Relation::Function).to_vec(),
        count: RowCount(count as usize),
        cells: (0..count)
            .flat_map(|row| {
                [
                    Cell::Node(node_id(file, Position(row))),
                    Cell::Symbol(other),
                    Cell::File(file),
                    Cell::Span(Span {
                        start: Offset(row),
                        end: Offset(row + 1),
                    }),
                ]
            })
            .collect(),
    };
    let right = Table {
        kinds: kinds(Relation::SemanticChild).to_vec(),
        count: RowCount((count * 10) as usize),
        cells: (0..count)
            .flat_map(|row| {
                (0..10).flat_map(move |repeated| {
                    [
                        Cell::Node(node_id(file, Position(row))),
                        Cell::Node(node_id(file, Position(row + 1000))),
                        Cell::Number(Integer(repeated)),
                        Cell::Symbol(if hit && repeated != 9 { body } else { other }),
                        Cell::File(file),
                    ]
                })
            })
            .collect(),
    };
    [
        stored(Relation::Function, left),
        stored(Relation::SemanticChild, right),
    ]
}
pub(in super::super) fn expected(count: u32) -> Vec<Cell> {
    (0..count)
        .flat_map(|row| {
            [
                Cell::Node(node_id(FileId(0), Position(row))),
                Cell::File(FileId(0)),
                Cell::Node(node_id(FileId(0), Position(row + 1000))),
            ]
        })
        .collect()
}
