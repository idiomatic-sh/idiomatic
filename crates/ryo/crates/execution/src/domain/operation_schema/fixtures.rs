//! Canonical typed operation fixture data; no evaluation or execution owner.
use ryo_language::domain::semantics::types::RenderContext;
use ryo_relations::domain::{
    relations::{
        service::node_id,
        types::{CellKind, FileId, Flag, Integer, Offset, Position, Relation, Span, Value as Cell},
    },
    terms::types::Symbol,
};
use ryo_values::domain::{
    plan_parts::types::{
        CapturePattern, ColumnIndex, Direction, FlagState, HostedNumber, Input as Source,
        JoinColumn, LiteralText, Operand, ParameterId, QueryLiteral, RepeatedRows, Side, TableKey,
        WholeNumber,
    },
    plans::types::Operation,
    query_bindings::types::{BindingCell, BindingTable, ParameterBinding},
    rows::{
        service::push_row,
        types::{RowCount, Table},
    },
};
/// A named column index for the fixture's mixed row.
pub(crate) fn col(value: u32) -> ColumnIndex {
    ColumnIndex(value)
}
/// A stable real file/position identity, not a composite graph encoding.
pub(crate) fn node(value: u32) -> Cell {
    Cell::Node(node_id(FileId(0), Position(value)))
}
/// The irrelevant logical source used by direct operator subjects.
const FROM: Source = Source::Relation(Relation::Function);
/// A two-sided projection with repeats, order and unlike kinds.
pub(crate) fn output() -> Vec<JoinColumn> {
    vec![
        JoinColumn {
            side: Side::Right,
            column: col(5),
        },
        JoinColumn {
            side: Side::Left,
            column: col(1),
        },
        JoinColumn {
            side: Side::Left,
            column: col(1),
        },
    ]
}
/// A typed row, whose schema contains every primitive engine kind.
pub(crate) fn mixed(symbol: Symbol) -> Table {
    let mut table = Table {
        kinds: vec![
            CellKind::Number,
            CellKind::Symbol,
            CellKind::Node,
            CellKind::Span,
            CellKind::Flag,
            CellKind::File,
        ],
        ..Table::default()
    };
    push_row(
        &mut table,
        &[
            Cell::Number(Integer(1)),
            Cell::Symbol(symbol),
            node(0),
            Cell::Span(Span {
                start: Offset(0),
                end: Offset(0),
            }),
            Cell::Flag(Flag::Raised),
            Cell::File(FileId(0)),
        ],
    );
    table
}
/// One directed edge with file partition, numeric order and retained text identity.
pub(crate) fn edges(symbol: Symbol) -> Table {
    let mut table = Table {
        kinds: vec![
            CellKind::Node,
            CellKind::Node,
            CellKind::Number,
            CellKind::Symbol,
            CellKind::File,
        ],
        ..Table::default()
    };
    push_row(
        &mut table,
        &[
            node(0),
            node(1),
            Cell::Number(Integer(2)),
            Cell::Symbol(symbol),
            Cell::File(FileId(0)),
        ],
    );
    table
}
/// Scalar and collection bindings, including all accepted empty-list encodings.
pub(crate) fn bindings() -> Vec<ParameterBinding> {
    vec![
        ParameterBinding::Text(LiteralText("fixed".into())),
        ParameterBinding::Number(WholeNumber(3)),
        ParameterBinding::Flag(FlagState::Raised),
        ParameterBinding::Texts(Vec::new()),
        ParameterBinding::Numbers(Vec::new()),
        ParameterBinding::TextSets(Vec::new()),
        ParameterBinding::Tables(vec![BindingTable(vec![BindingCell {
            key: LiteralText("name".into()),
            value: LiteralText("bound".into()),
        }])]),
        ParameterBinding::TextSets(vec![vec![LiteralText("alpha/beta".into())]]),
    ]
}
/// Unary row operators, including zero-width projection and scalar extensions.
fn unary() -> Vec<Operation> {
    vec![
        Operation::Select {
            from: FROM,
            comparisons: Vec::new(),
            kept: vec![col(1), col(0), col(1)],
        },
        Operation::Project {
            from: FROM,
            columns: Vec::new(),
            distinct: RepeatedRows::Kept,
        },
        Operation::Extend {
            from: FROM,
            values: vec![
                Operand::Literal(QueryLiteral::Number(Integer(4))),
                Operand::Literal(QueryLiteral::Text(LiteralText("new".into()))),
                Operand::Parameter(ParameterId(0)),
                Operand::Parameter(ParameterId(1)),
                Operand::Parameter(ParameterId(2)),
            ],
        },
        Operation::GroupCount {
            from: FROM,
            keys: vec![col(1), col(0), col(1)],
            distinct: Some(col(4)),
        },
        Operation::GroupDigest {
            from: FROM,
            keys: vec![col(1), col(1)],
            over: col(0),
        },
        Operation::Capture {
            from: FROM,
            path: col(1),
            pattern: CapturePattern("{parent}/{child}".into()),
        },
        Operation::Rank {
            from: FROM,
            column: col(1),
            sets: ParameterId(7),
        },
        Operation::Replace {
            from: FROM,
            text: col(1),
            pattern: LiteralText("alpha".into()),
            replacement: LiteralText("replaced".into()),
        },
        Operation::Words {
            from: FROM,
            text: col(1),
        },
        Operation::SpanBetween {
            from: FROM,
            start: col(3),
            end: col(3),
        },
        Operation::SourceMatches {
            from: FROM,
            span: col(3),
            base_line: None,
            pattern: LiteralText("a".into()),
            capture: None,
            capture_text: false,
            capture_line: false,
        },
        Operation::RenderSyntax {
            from: FROM,
            node: col(2),
            context: RenderContext::PositionalArgument,
        },
        Operation::FitsRendered {
            from: FROM,
            texts: vec![col(1)],
            literals: vec![LiteralText("".into()), LiteralText("".into())],
            span: col(3),
            max_line: 88,
            tab_width: 4,
        },
        Operation::SpanBounds {
            from: FROM,
            span: col(3),
        },
    ]
}
/// Two-input row operators use the same input kinds and explicit ordered projection.
fn binary() -> Vec<Operation> {
    vec![
        Operation::Join {
            left: FROM,
            right: FROM,
            left_on: vec![col(1)],
            right_on: vec![col(1)],
            output: output(),
            filter: Vec::new(),
            view: Vec::new(),

            reach: None,
        },
        Operation::Union {
            left: FROM,
            right: FROM,
        },
        Operation::Intersection {
            left: FROM,
            right: FROM,
        },
        Operation::Difference {
            left: FROM,
            right: FROM,
        },
        Operation::Antijoin {
            left: FROM,
            right: FROM,
            left_on: vec![col(1)],
            right_on: vec![col(1)],
        },
        Operation::OrderedMatch {
            left: FROM,
            right: FROM,
            left_on: vec![col(1)],
            right_on: vec![col(1)],
            left_order: col(0),
            right_order: col(0),
            output: output(),
        },
    ]
}
/// Traversals retain root metadata and graph partitions without graph-dependent schema.
pub(crate) fn graph_operations() -> Vec<Operation> {
    vec![
        Operation::Axis {
            from: FROM,
            edges: FROM,
            domain: FROM,
            direction: Direction::Descendant,
            id: col(2),
            parent: col(0),
            child: col(1),
            domain_id: col(0),
            domain_filter: Vec::new(),
            domain_view: Vec::new(),
            output: Vec::new(),
        },
        Operation::Closure {
            edges: FROM,
            keys: vec![col(4), col(4)],
            source: col(0),
            destination: col(1),
        },
        Operation::Cycles {
            edges: FROM,
            source: col(0),
            destination: col(1),
        },
        Operation::OrderedDfs {
            from: FROM,
            edges: FROM,
            root: col(2),
            source: col(0),
            destination: col(1),
            order: col(2),
            edge: col(3),
            root_keys: vec![col(5)],
            edge_keys: vec![col(4)],
        },
    ]
}
/// Every current Operation variant exactly once, including a real Hosted request source.
pub(crate) fn operations() -> Vec<Operation> {
    let mut values = unary();
    values.extend(binary());
    values.extend(graph_operations());
    values.push(Operation::Rows {
        parameter: ParameterId(6),
        keys: vec![TableKey("name".into()), TableKey("name".into())],
    });
    values.push(Operation::Hosted {
        relation: HostedNumber(0),
    });
    values
}
/// Actual typed inputs of each operator, with cardinality varied independently of schema.
pub(crate) fn tables(operation: &Operation, symbol: Symbol, populated: bool) -> Vec<Table> {
    let mut first = mixed(symbol);
    let mut second = first.clone();
    second.cells[0] = Cell::Number(Integer(2));
    let graph = edges(symbol);
    let mut values = match operation {
        Operation::Axis { .. } => {
            let mut domain = Table {
                kinds: vec![CellKind::Node],
                ..Table::default()
            };
            push_row(&mut domain, &[node(1)]);
            vec![first, graph, domain]
        }
        Operation::Closure { .. } | Operation::Cycles { .. } => vec![graph],
        Operation::OrderedDfs { .. } => vec![first, graph],
        Operation::Rows { .. } => Vec::new(),
        Operation::Hosted { .. } => vec![Table {
            kinds: vec![CellKind::Number, CellKind::Symbol, CellKind::File],
            count: RowCount(1),
            cells: vec![
                Cell::Number(Integer(1)),
                Cell::Symbol(symbol),
                Cell::File(FileId(0)),
            ],
        }],
        Operation::Join { .. }
        | Operation::Union { .. }
        | Operation::Intersection { .. }
        | Operation::Difference { .. }
        | Operation::Antijoin { .. }
        | Operation::OrderedMatch { .. } => vec![first, second],
        _ => vec![std::mem::take(&mut first)],
    };
    if !populated {
        for table in &mut values {
            table.cells.clear();
            table.count = RowCount(0);
        }
    }
    values
}
