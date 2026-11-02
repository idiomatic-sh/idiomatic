//! Traversal schemas preserve representation-sensitive projections and reject bad metadata.

use super::value::TraversalSchema;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::operating::{errors::OperatingError, types::OperatorContext};
use crate::domain::operation_schema::types::InputSchema;
use crate::domain::operator::service::compute;
use crate::domain::table_cells::types::{TreeForm, TreeRows};
use crate::domain::table_model::{
    errors::TableModelError,
    service::{kinds_of, push_row},
    types::{Rows, Table},
};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::elements::{
    service::{enter_node, leave_node, table_builder, token},
    types::Elements,
};
use ryo_language::domain::relations::service::node_id;
use ryo_language::domain::relations::types::{
    CellKind, FileId, KindBits, Offset, Position, Relation, Value as Cell,
};
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Direction, Input as Source, JoinColumn, Side,
};
use ryo_values::domain::plans::types::Operation;

/// One native or invalid traversal contract.
#[derive(Clone, Copy)]
enum Scenario {
    NativeAxis,
    NativeJoinView,
    ComputedProjection,
    ComputedDeep,
    BadOrder,
    BadOutput,
    BadKeys,
    BadRootKeys,
    MissingEndpoint,
    EndpointKind,
    MissingInput,
}
/// The representation or invalid column selection being exercised.
type Input = Scenario;
/// How many kinds a valid native projection returns.
type Value = usize;
type Failure = OperatingError;
/// A logical source irrelevant to direct evaluator inputs.
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
fn ordered() -> Operation {
    Operation::OrderedMatch {
        left: FROM,
        right: FROM,
        left_on: vec![column(0)],
        right_on: vec![column(0)],
        left_order: column(1),
        right_order: column(1),
        output: vec![JoinColumn {
            side: Side::Left,
            column: column(0),
        }],
    }
}
fn dfs() -> Operation {
    Operation::OrderedDfs {
        from: FROM,
        edges: FROM,
        root: column(0),
        source: column(0),
        destination: column(1),
        order: column(2),
        edge: column(1),
        root_keys: Vec::new(),
        edge_keys: Vec::new(),
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
        ryo_language::domain::terms::types::NameText("root"),
    );
    ryo_language::domain::completion::service::name_kind(
        &mut finish,
        KindBits(2),
        ryo_language::domain::terms::types::NameText("token"),
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

/// Compare actual native execution kinds with the same metadata-only operation.
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
            .map(|row| InputSchema {
                kinds: kinds_of(row),
                tree: match row {
                    Rows::Tree(tree) => Some(tree.form),
                    _ => None,
                },
            })
            .collect();
        let answer =
            crate::domain::operation_schema::service::schema_of(&operation, &schemas, &[])?;
        let mut scratch = OperatorScratch::default();
        let mut memo = Vec::new();
        let mut output = Table::default();
        let mut context = OperatorContext {
            prepared_kinds: None,
            prepared_matchers: None,
            stored_indexes: None,
            bindings: &[],
            symbols: (&mut symbols).into(),
            source: SourceView(b"x"),
            paths: &[],
            arenas: &[],
            scratch: &mut scratch,
            memo: &mut memo,
        };
        compute(&operation, &rows, &mut context, &mut output)?;
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

fn subject(scenario: Input) -> Result<Value, Failure> {
    if matches!(scenario, Scenario::NativeAxis | Scenario::NativeJoinView) {
        return native(scenario);
    }
    let kinds = [CellKind::Node, CellKind::Number, CellKind::Number];
    let mut schemas = [InputSchema {
        kinds: &kinds,
        tree: None,
    }; 3];
    let graph_kinds = [CellKind::Node, CellKind::Node, CellKind::Number];
    if matches!(scenario, Scenario::BadRootKeys | Scenario::MissingEndpoint) {
        schemas[1].kinds = &graph_kinds;
    }
    let mut operation = axis();
    match scenario {
        Scenario::ComputedProjection => {
            if let Operation::Axis { output, .. } = &mut operation {
                output.push(JoinColumn {
                    side: Side::Right,
                    column: column(0),
                });
            }
        }
        Scenario::ComputedDeep => {
            if let Operation::Axis { direction, .. } = &mut operation {
                *direction = Direction::Deep;
            }
        }
        Scenario::BadOrder => {
            operation = ordered();
            if let Operation::OrderedMatch { left_order, .. } = &mut operation {
                *left_order = column(0);
            }
        }
        Scenario::BadOutput => {
            operation = ordered();
            if let Operation::OrderedMatch { output, .. } = &mut operation {
                output[0].column = column(99);
            }
        }
        Scenario::BadKeys => {
            operation = ordered();
            if let Operation::OrderedMatch {
                left_on, right_on, ..
            } = &mut operation
            {
                *left_on = vec![column(99)];
                *right_on = vec![column(99)];
            }
        }
        Scenario::BadRootKeys => {
            operation = dfs();
            if let Operation::OrderedDfs { root_keys, .. } = &mut operation {
                root_keys.push(column(0));
            }
        }
        Scenario::MissingEndpoint => {
            operation = dfs();
            if let Operation::OrderedDfs { destination, .. } = &mut operation {
                *destination = column(99);
            }
        }
        Scenario::EndpointKind => operation = dfs(),
        Scenario::MissingInput => {
            return TraversalSchema::new(&[])
                .of(&operation, &[])
                .map(|kinds| kinds.len());
        }
        Scenario::NativeAxis | Scenario::NativeJoinView => unreachable!(),
    }
    TraversalSchema::new(&schemas)
        .of(&operation, &[])
        .map(|kinds| kinds.len())
}

/// Native views and explicit schema refusals do not depend on graph rows.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("ordered_dfs_checks_endpoint_kinds"),
        word: Word::Refuses,
        input: || Scenario::EndpointKind,
        expect: Expect::Error(OperatingError::ShapeMismatch),
    },
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
    Case {
        name: CaseName("computed_axis_projection_remains_unsupported"),
        word: Word::Refuses,
        input: || Scenario::ComputedProjection,
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("computed_deep_axis_remains_unsupported"),
        word: Word::Refuses,
        input: || Scenario::ComputedDeep,
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("ordered_match_requires_numeric_order"),
        word: Word::Refuses,
        input: || Scenario::BadOrder,
        expect: Expect::Error(OperatingError::ShapeMismatch),
    },
    Case {
        name: CaseName("ordered_match_checks_output_columns"),
        word: Word::Refuses,
        input: || Scenario::BadOutput,
        expect: Expect::Error(OperatingError::ShapeMismatch),
    },
    Case {
        name: CaseName("ordered_match_checks_missing_keys_on_both_sides"),
        word: Word::Refuses,
        input: || Scenario::BadKeys,
        expect: Expect::Error(OperatingError::ShapeMismatch),
    },
    Case {
        name: CaseName("ordered_dfs_checks_root_key_count"),
        word: Word::Refuses,
        input: || Scenario::BadRootKeys,
        expect: Expect::Error(OperatingError::ShapeMismatch),
    },
    Case {
        name: CaseName("ordered_dfs_checks_endpoint_columns"),
        word: Word::Refuses,
        input: || Scenario::MissingEndpoint,
        expect: Expect::Error(OperatingError::Table(TableModelError::NoSuchColumn {
            index: 99,
        })),
    },
    Case {
        name: CaseName("traversal_missing_input_remains_explicit"),
        word: Word::Refuses,
        input: || Scenario::MissingInput,
        expect: Expect::Error(OperatingError::MissingInput { position: 1 }),
    },
];
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
