//! Proves every operation schema against its actual empty and populated evaluator.

use crate::application::file_dag::cross_execution::compute;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::features::service::{catalog, kind_of};
use crate::domain::operating::{errors::OperatingError, types::OperatorContext};
use crate::domain::operation_schema::service::schema_of;
use crate::domain::operation_schema::types::InputSchema;
use crate::domain::table_model::types::Rows;
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::symbols::{
    service::intern_derived,
    types::{ByteSlice, Symbols},
};
use ryo_language::domain::terms::types::NameText;
use ryo_values::domain::plans::types::{Feature, Operation, OperationKind};

use crate::domain::table_model::service::push_row;
use crate::domain::table_model::types::{
    BindingCell, BindingTable, ParameterBinding, RowCount, Table,
};
use ryo_language::domain::relations::service::node_id;
use ryo_language::domain::relations::types::{
    CellKind, FileId, Flag, Integer, Offset, Position, Relation, Span, Value as Cell,
};
use ryo_language::domain::terms::types::Symbol;
use ryo_values::domain::plan_parts::types::{
    CapturePattern, ColumnIndex, Direction, FlagState, HostedNumber, Input as Source, JoinColumn,
    LiteralText, Operand, ParameterId, QueryLiteral, RepeatedRows, Side, TableKey, WholeNumber,
};

/// A genuine populated/empty evaluator comparison across every current operator.
type Input = bool;
/// The number of distinct operator variants whose shapes were compared.
type Value = usize;
type Failure = OperatingError;

fn subject(populated: Input) -> Result<Value, Failure> {
    let mut symbols = Symbols::default();
    let symbol = intern_derived(&mut symbols, ByteSlice(b""), NameText("alpha/beta"));
    let bindings = bindings();
    let mut scratch = OperatorScratch::default();
    let mut memo = Vec::new();
    let mut variants = std::collections::HashSet::new();
    for operation in operations() {
        assert!(variants.insert(std::mem::discriminant(&operation)));
        let tables = tables(&operation, symbol, populated);
        let schemas: Vec<_> = tables
            .iter()
            .map(|table| InputSchema {
                kinds: &table.kinds,
                tree: None,
            })
            .collect();
        let actual = schema_of(&operation, &schemas, &bindings);
        let rows: Vec<_> = tables.iter().map(Rows::Computed).collect();
        let mut output = Table::default();
        let mut context = OperatorContext {
            semantic: None,
            prepared_kinds: None,
            prepared_matchers: None,
            bindings: &bindings,
            symbols: (&mut symbols).into(),
            source: SourceView(b""),
            paths: &[],
            arenas: &[],
            scratch: &mut scratch,
            memo: &mut memo,
        };
        let evaluated = compute(&operation, &rows, &mut context, &mut output);
        evaluated?;
        assert_eq!(
            actual?, output.kinds,
            "{operation:?}, populated={populated}"
        );
        if matches!(operation, Operation::Hosted { .. }) {
            assert_eq!(output.cells, tables[0].cells, "real request-source rows");
        }
    }
    Ok(variants.len())
}

/// Both input cardinalities exercise every registered concrete operation fixture.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("all_empty_operator_shapes_match_evaluation"),
        word: Word::Succeeds,
        input: || false,
        expect: Expect::Value(24),
    },
    Case {
        name: CaseName("all_populated_operator_shapes_match_evaluation"),
        word: Word::Succeeds,
        input: || true,
        expect: Expect::Value(24),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn every_declared_kind_has_one_evaluator_fixture_and_one_feature() {
    let declared: std::collections::HashSet<_> = OperationKind::ALL.iter().copied().collect();
    assert_eq!(
        declared.len(),
        OperationKind::ALL.len(),
        "duplicate kind declaration"
    );
    let mut fixtures = std::collections::HashSet::new();
    for operation in operations() {
        assert!(
            fixtures.insert(kind_of(&operation)),
            "duplicate operation fixture"
        );
    }
    assert_eq!(fixtures, declared, "missing operator fixture");
    let features: Vec<_> = catalog()
        .into_iter()
        .filter_map(|feature| match feature {
            Feature::Operation(kind) => Some(kind),
            Feature::Family(_) | Feature::Direction(_) | Feature::Relation(_) => None,
        })
        .collect();
    assert_eq!(
        features.len(),
        declared.len(),
        "duplicate or omitted feature"
    );
    assert_eq!(
        features
            .into_iter()
            .collect::<std::collections::HashSet<_>>(),
        declared
    );
}

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
        Operation::Hosted { .. } => vec![first],
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

#[test]
fn demand_join_schema_uses_virtual_partition_source_destination_projection() {
    use ryo_values::domain::plan_parts::types::ReachLookup;
    let operation = Operation::Join {
        left: FROM,
        right: FROM,
        left_on: vec![ColumnIndex(0)],
        right_on: vec![ColumnIndex(2)],
        output: vec![
            JoinColumn {
                side: Side::Right,
                column: ColumnIndex(0),
            },
            JoinColumn {
                side: Side::Right,
                column: ColumnIndex(1),
            },
            JoinColumn {
                side: Side::Right,
                column: ColumnIndex(0),
            },
        ],
        filter: vec![],
        view: vec![],
        reach: Some(ReachLookup {
            source: ColumnIndex(1),
            destination: ColumnIndex(0),
            keys: vec![ColumnIndex(2)],
            columns: vec![ColumnIndex(2), ColumnIndex(0), ColumnIndex(1)],
        }),
    };
    let inputs = [
        InputSchema {
            kinds: &[CellKind::Node],
            tree: None,
        },
        InputSchema {
            kinds: &[CellKind::Symbol, CellKind::Node, CellKind::File],
            tree: None,
        },
    ];
    assert_eq!(
        schema_of(&operation, &inputs, &[]).expect("virtual kinds"),
        [CellKind::Symbol, CellKind::File, CellKind::Symbol]
    );
}

#[test]
fn hosted_without_its_request_source_is_missing_not_an_unsupported_operation() {
    let operation = Operation::Hosted {
        relation: HostedNumber(0),
    };
    assert_eq!(
        schema_of(&operation, &[], &[]),
        Err(OperatingError::MissingInput { position: 1 })
    );
    let mut symbols = Symbols::default();
    let mut context = OperatorContext {
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        scratch: &mut OperatorScratch::default(),
        memo: &mut Vec::new(),
    };
    assert_eq!(
        compute(&operation, &[], &mut context, &mut Table::default()),
        Err(OperatingError::MissingInput { position: 1 })
    );
}
