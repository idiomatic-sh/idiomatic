//! Independent authored rows and expectations for every operation class.
use expectations::Expected;
use expectations::Fixture;
use expectations::cells;
use ryo_language::domain::semantics::types::RenderContext;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Direction;
use ryo_values::domain::plan_parts::types::HostedNumber;
use ryo_values::domain::plan_parts::types::JoinColumn;
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::plan_parts::types::Operand;
use ryo_values::domain::plan_parts::types::ParameterId;
use ryo_values::domain::plan_parts::types::QueryLiteral;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::plan_parts::types::TableKey;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_bindings::types::BindingCell;
use ryo_values::domain::query_bindings::types::BindingTable;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::query_operators::types::OperationKind;
pub mod expectations;
pub mod inputs;
pub mod values;
use values::{FROM, RIGHT, DOMAIN, n, span, node, table, numbers, matrix, text};

/// The authored rows, bindings and expected answer of one operation kind.
#[must_use]
pub fn fixture(kind: OperationKind) -> Fixture {
    let mut symbols = Symbols::default();
    let mut bindings = vec![];
    let (operation, inputs, expected) = match kind {
        OperationKind::Select => (
            Operation::Select {
                from: FROM,
                comparisons: vec![],
                kept: vec![],
            },
            vec![numbers(&[2, 1, 2])],
            cells(vec![vec![n(2)], vec![n(1)], vec![n(2)]]),
        ),
        OperationKind::Project => (
            Operation::Project {
                from: FROM,
                columns: vec![ColumnIndex(0)],
                distinct: RepeatedRows::Dropped,
            },
            vec![matrix(&[&[2, 9], &[1, 8], &[2, 7]])],
            cells(vec![vec![n(2)], vec![n(1)]]),
        ),
        OperationKind::Extend => (
            Operation::Extend {
                from: FROM,
                values: vec![Operand::Literal(QueryLiteral::Number(Integer(5)))],
            },
            vec![numbers(&[2, 1])],
            cells(vec![vec![n(2), n(5)], vec![n(1), n(5)]]),
        ),
        OperationKind::Join => (
            Operation::Join {
                left: FROM,
                right: RIGHT,
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
                output: vec![
                    JoinColumn {
                        side: Side::Left,
                        column: ColumnIndex(1),
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
            vec![
                matrix(&[&[2, 8], &[1, 7]]),
                matrix(&[&[1, 4], &[2, 5], &[2, 6]]),
            ],
            cells(vec![vec![n(8), n(5)], vec![n(8), n(6)], vec![n(7), n(4)]]),
        ),
        OperationKind::Union => (
            Operation::Union {
                left: FROM,
                right: RIGHT,
            },
            vec![numbers(&[2, 1, 2]), numbers(&[3, 1])],
            cells(vec![vec![n(2)], vec![n(1)], vec![n(3)]]),
        ),
        OperationKind::Intersection => (
            Operation::Intersection {
                left: FROM,
                right: RIGHT,
            },
            vec![numbers(&[2, 1, 2]), numbers(&[2])],
            cells(vec![vec![n(2)]]),
        ),
        OperationKind::Difference => (
            Operation::Difference {
                left: FROM,
                right: RIGHT,
            },
            vec![numbers(&[2, 1, 2]), numbers(&[2])],
            cells(vec![vec![n(1)]]),
        ),
        OperationKind::Antijoin => (
            Operation::Antijoin {
                left: FROM,
                right: RIGHT,
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
            },
            vec![numbers(&[2, 1, 2]), numbers(&[1])],
            cells(vec![vec![n(2)], vec![n(2)]]),
        ),
        OperationKind::GroupCount => (
            Operation::GroupCount {
                from: FROM,
                keys: vec![ColumnIndex(0)],
                distinct: None,
            },
            vec![numbers(&[2, 2, 1])],
            cells(vec![vec![n(2), n(2)], vec![n(1), n(1)]]),
        ),
        OperationKind::GroupDigest => (
            Operation::GroupDigest {
                from: FROM,
                keys: vec![ColumnIndex(0)],
                over: ColumnIndex(1),
            },
            vec![matrix(&[&[2, 10], &[2, 20], &[1, 20], &[1, 10]])],
            vec![
                vec![
                    Expected::Cell(n(2)),
                    Expected::Digest(0),
                    Expected::Cell(n(2)),
                ],
                vec![
                    Expected::Cell(n(1)),
                    Expected::Digest(0),
                    Expected::Cell(n(2)),
                ],
            ],
        ),
        OperationKind::Capture => {
            let value = text("docs/page.md", &mut symbols);
            (
                Operation::Capture {
                    from: FROM,
                    path: ColumnIndex(0),
                    pattern: ryo_values::domain::plan_parts::types::CapturePattern(
                        "docs/{page}.md".into(),
                    ),
                },
                vec![table(&[CellKind::Symbol], &[vec![value]])],
                vec![vec![Expected::Text("docs/page.md"), Expected::Text("page")]],
            )
        }
        OperationKind::Rank => {
            let value = text("beta", &mut symbols);
            bindings.push(ParameterBinding::TextSets(vec![
                vec![LiteralText("alpha".into())],
                vec![LiteralText("beta".into())],
            ]));
            (
                Operation::Rank {
                    from: FROM,
                    column: ColumnIndex(0),
                    sets: ParameterId(0),
                },
                vec![table(&[CellKind::Symbol], &[vec![value]])],
                vec![vec![Expected::Text("beta"), Expected::Cell(n(1))]],
            )
        }
        OperationKind::Replace => {
            let value = text("aba", &mut symbols);
            (
                Operation::Replace {
                    from: FROM,
                    text: ColumnIndex(0),
                    pattern: LiteralText("a".into()),
                    replacement: LiteralText("$$".into()),
                },
                vec![table(&[CellKind::Symbol], &[vec![value]])],
                vec![vec![Expected::Text("aba"), Expected::Text("$$b$$")]],
            )
        }
        OperationKind::OrderedMatch => (
            Operation::OrderedMatch {
                left: FROM,
                right: RIGHT,
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
                left_order: ColumnIndex(1),
                right_order: ColumnIndex(1),
                output: vec![
                    JoinColumn {
                        side: Side::Left,
                        column: ColumnIndex(1),
                    },
                    JoinColumn {
                        side: Side::Right,
                        column: ColumnIndex(1),
                    },
                ],
            },
            vec![matrix(&[&[1, 3], &[1, 1]]), matrix(&[&[1, 4], &[1, 2]])],
            cells(vec![vec![n(1), n(2)], vec![n(3), n(4)]]),
        ),
        OperationKind::Words => {
            let value = text("parseXML_2nd", &mut symbols);
            (
                Operation::Words {
                    from: FROM,
                    text: ColumnIndex(0),
                },
                vec![table(&[CellKind::Symbol], &[vec![value]])],
                vec![
                    vec![Expected::Text("parseXML_2nd"), Expected::Text("parse")],
                    vec![Expected::Text("parseXML_2nd"), Expected::Text("xml")],
                    vec![Expected::Text("parseXML_2nd"), Expected::Text("2nd")],
                ],
            )
        }
        OperationKind::Axis => (
            Operation::Axis {
                from: FROM,
                edges: RIGHT,
                domain: DOMAIN,
                direction: Direction::Descendant,
                id: ColumnIndex(0),
                parent: ColumnIndex(0),
                child: ColumnIndex(1),
                domain_id: ColumnIndex(0),
                domain_filter: vec![],
                domain_view: vec![],
                output: vec![],
            },
            vec![
                table(&[CellKind::Node], &[vec![node(1)]]),
                table(
                    &[CellKind::Node; 2],
                    &[vec![node(1), node(2)], vec![node(2), node(3)]],
                ),
                table(&[CellKind::Node], &[vec![node(2)], vec![node(3)]]),
            ],
            cells(vec![vec![node(1), node(2)], vec![node(1), node(3)]]),
        ),
        OperationKind::Closure => (
            Operation::Closure {
                edges: FROM,
                keys: vec![],
                source: ColumnIndex(0),
                destination: ColumnIndex(1),
            },
            vec![matrix(&[&[1, 2], &[2, 3]])],
            cells(vec![vec![n(1), n(2)], vec![n(1), n(3)], vec![n(2), n(3)]]),
        ),
        OperationKind::Cycles => (
            Operation::Cycles {
                edges: FROM,
                source: ColumnIndex(0),
                destination: ColumnIndex(1),
            },
            vec![matrix(&[&[2, 1], &[1, 2], &[3, 4]])],
            cells(vec![vec![n(1), n(1)], vec![n(2), n(2)]]),
        ),
        OperationKind::Rows => {
            bindings.push(ParameterBinding::Tables(vec![BindingTable(vec![
                BindingCell {
                    key: LiteralText("key".into()),
                    value: LiteralText("value".into()),
                },
            ])]));
            (
                Operation::Rows {
                    parameter: ParameterId(0),
                    keys: vec![TableKey("key".into())],
                },
                vec![],
                vec![vec![Expected::Text("value")]],
            )
        }
        OperationKind::Hosted => (
            Operation::Hosted {
                relation: HostedNumber(0),
            },
            vec![numbers(&[8, 7])],
            cells(vec![vec![n(8)], vec![n(7)]]),
        ),
        OperationKind::SpanBetween => (
            Operation::SpanBetween {
                from: FROM,
                start: ColumnIndex(0),
                end: ColumnIndex(1),
            },
            vec![table(&[CellKind::Span; 2], &[vec![span(0, 1), span(2, 3)]])],
            cells(vec![vec![span(0, 1), span(2, 3), span(0, 3)]]),
        ),
        OperationKind::SourceMatches => (
            Operation::SourceMatches {
                from: FROM,
                span: ColumnIndex(0),
                base_line: None,
                pattern: LiteralText("a".into()),
                capture: None,
                capture_text: ryo_values::domain::query_operators::types::TextOutput::Omitted,
                capture_line: ryo_values::domain::query_operators::types::LineOutput::Omitted,
            },
            vec![table(&[CellKind::Span], &[vec![span(0, 3)]])],
            cells(vec![
                vec![span(0, 3), span(0, 1)],
                vec![span(0, 3), span(2, 3)],
            ]),
        ),
        OperationKind::RenderSyntax => (
            Operation::RenderSyntax {
                from: FROM,
                node: ColumnIndex(0),
                context: RenderContext::PositionalArgument,
            },
            vec![table(&[CellKind::Node], &[vec![node(0)]])],
            vec![vec![Expected::Cell(node(0)), Expected::Text("a")]],
        ),
        OperationKind::FitsRendered => (
            Operation::FitsRendered {
                from: FROM,
                texts: vec![ColumnIndex(0)],
                literals: vec![LiteralText("".into()), LiteralText("".into())],
                span: ColumnIndex(1),
                max_line: ryo_values::domain::query_operators::types::MaxLine(88),
                tab_width: ryo_values::domain::query_operators::types::TabWidth(4),
            },
            vec![table(
                &[CellKind::Symbol, CellKind::Span],
                &[vec![text("a", &mut symbols), span(0, 3)]],
            )],
            vec![vec![Expected::Text("a"), Expected::Cell(span(0, 3))]],
        ),
        OperationKind::SpanBounds => (
            Operation::SpanBounds {
                from: FROM,
                span: ColumnIndex(0),
            },
            vec![table(&[CellKind::Span], &[vec![span(0, 3)]])],
            cells(vec![vec![span(0, 3), n(0), n(3)]]),
        ),
        OperationKind::OrderedDfs => (
            Operation::OrderedDfs {
                from: FROM,
                edges: RIGHT,
                root: ColumnIndex(0),
                source: ColumnIndex(0),
                destination: ColumnIndex(1),
                order: ColumnIndex(2),
                edge: ColumnIndex(3),
                root_keys: vec![],
                edge_keys: vec![],
            },
            vec![numbers(&[1]), matrix(&[&[1, 2, 0, 9], &[2, 3, 0, 10]])],
            cells(vec![
                vec![n(1), n(1), n(2), n(9), n(0)],
                vec![n(1), n(2), n(3), n(10), n(1)],
            ]),
        ),
    };
    Fixture {
        operation,
        inputs,
        bindings,
        symbols,
        expected,
    }
}
