//! One head per Operation variant of the corpus.
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{
    CapturePattern, ColumnIndex, Comparison, Direction, HostedNumber, Input, JoinColumn,
    LiteralText, Operand, ParameterId, QueryLiteral, RepeatedRows, Side, TableKey, Test,
};
use ryo_values::domain::plans::types::Operation;
// Text the plans carry in; it enters the engine here, as authored query text does.
pub(crate) const CAPTURE_PATTERN: &str = "{parent}/{child}";
pub(crate) const ROW_KEY: &str = "key";
pub(crate) const TEXT_A: &str = "a";
pub(crate) const TEXT_B: &str = "b";
pub(crate) const TEXT_EMPTY: &str = "";
/// The function relation: id Node, name Symbol, file File, span Span.
pub(in super::super::super) const FUNCTION: Relation = Relation::Function;
/// The syntax-child relation: parent Node, child Node, ordinal Number, field Symbol, file File.
pub(in super::super::super) const EDGES: Relation = Relation::SyntaxChild;
pub(crate) fn from(relation: Relation) -> Input {
    Input::Relation(relation)
}
pub(crate) fn output() -> Vec<JoinColumn> {
    vec![
        JoinColumn {
            side: Side::Left,
            column: ColumnIndex(0),
        },
        JoinColumn {
            side: Side::Right,
            column: ColumnIndex(1),
        },
    ]
}
pub(crate) fn equal(column: ColumnIndex) -> Comparison {
    Comparison {
        test: Test::Equal,
        left: Operand::Column(column),
        right: Operand::Column(column),
        origin: None,
    }
}
/// One head operation for every variant, over `FUNCTION` and `EDGES`, in no particular order.
pub(in super::super::super) fn operations() -> Vec<Operation> {
    let f = from(FUNCTION);
    let e = from(EDGES);
    vec![
        Operation::Select {
            from: f,
            comparisons: vec![],
            kept: vec![],
        },
        Operation::Project {
            from: f,
            columns: vec![ColumnIndex(0), ColumnIndex(0)],
            distinct: RepeatedRows::Kept,
        },
        Operation::Extend {
            from: f,
            values: vec![Operand::Literal(QueryLiteral::Number(
                ryo_relations::domain::relations::types::Integer(4),
            ))],
        },
        Operation::Join {
            left: f,
            right: f,
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
            output: output(),
            filter: vec![],
            view: vec![],
            reach: None,
        },
        Operation::Union { left: f, right: f },
        Operation::Intersection { left: f, right: f },
        Operation::Difference { left: f, right: f },
        Operation::Antijoin {
            left: f,
            right: f,
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
        },
        Operation::GroupCount {
            from: f,
            keys: vec![ColumnIndex(0)],
            distinct: None,
        },
        Operation::GroupDigest {
            from: e,
            keys: vec![ColumnIndex(0)],
            over: ColumnIndex(2),
        },
        Operation::Capture {
            from: f,
            path: ColumnIndex(1),
            pattern: CapturePattern(CAPTURE_PATTERN.into()),
        },
        Operation::Rank {
            from: f,
            column: ColumnIndex(1),
            sets: ParameterId(0),
        },
        Operation::Replace {
            from: f,
            text: ColumnIndex(1),
            pattern: LiteralText(TEXT_A.into()),
            replacement: LiteralText(TEXT_B.into()),
        },
        Operation::OrderedMatch {
            left: e,
            right: e,
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
            left_order: ColumnIndex(2),
            right_order: ColumnIndex(2),
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
        Operation::Words {
            from: f,
            text: ColumnIndex(1),
        },
        Operation::Axis {
            from: f,
            edges: e,
            domain: f,
            direction: Direction::Child,
            id: ColumnIndex(0),
            parent: ColumnIndex(0),
            child: ColumnIndex(1),
            domain_id: ColumnIndex(0),
            domain_filter: vec![],
            domain_view: vec![],
            output: vec![],
        },
        Operation::Closure {
            edges: e,
            keys: vec![],
            source: ColumnIndex(0),
            destination: ColumnIndex(1),
        },
        Operation::OrderedDfs {
            from: f,
            edges: e,
            root: ColumnIndex(0),
            source: ColumnIndex(0),
            destination: ColumnIndex(1),
            order: ColumnIndex(2),
            edge: ColumnIndex(3),
            root_keys: vec![],
            edge_keys: vec![],
        },
        Operation::Cycles {
            edges: e,
            source: ColumnIndex(0),
            destination: ColumnIndex(1),
        },
        Operation::Rows {
            parameter: ParameterId(0),
            keys: vec![TableKey(ROW_KEY.into())],
        },
        Operation::Hosted {
            relation: HostedNumber(0),
        },
        Operation::SpanBetween {
            from: f,
            start: ColumnIndex(3),
            end: ColumnIndex(3),
        },
        Operation::SourceMatches {
            from: f,
            span: ColumnIndex(3),
            base_line: None,
            pattern: LiteralText(TEXT_A.into()),
            capture: None,
            capture_text: false,
            capture_line: false,
        },
        Operation::RenderSyntax {
            from: f,
            node: ColumnIndex(0),
            context: ryo_language::domain::semantics::types::RenderContext::PositionalArgument,
        },
        Operation::FitsRendered {
            from: f,
            texts: vec![ColumnIndex(1)],
            literals: vec![
                LiteralText(TEXT_EMPTY.into()),
                LiteralText(TEXT_EMPTY.into()),
            ],
            span: ColumnIndex(3),
            max_line: 88,
            tab_width: 4,
        },
        Operation::SpanBounds {
            from: f,
            span: ColumnIndex(3),
        },
    ]
}
