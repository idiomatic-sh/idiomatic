//! Traversal shape refusals are pure representation-sensitive metadata contracts.
use std::borrow::Cow;

use crate::domain::{
    operating::errors::OperatingError,
    operation_schema::{types::InputShape, types::traversal::value::TraversalShape},
};
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        plan_parts::types::{ColumnIndex, Direction, Input as Source, JoinColumn, Side},
        plans::types::Operation,
    },
};
/// A traversal over three inputs of node, number and number columns (the second input's kinds replaced by `second`), or over no
/// input at all.
struct Traversal {
    operation: Operation,
    second: &'static [CellKind],
    inputs: usize,
}
type Input = Traversal;
/// The kinds a valid native projection returns.
type Value = Cow<'static, [CellKind]>;
type Failure = OperatingError;
/// A logical source irrelevant to direct evaluator inputs.
const FROM: Source = Source::Relation(Relation::SyntaxTree);
/// The kinds of each input.
const KINDS: &[CellKind] = &[CellKind::Node, CellKind::Number, CellKind::Number];
/// The kinds of a graph input: two node endpoints and a number.
const GRAPH: &[CellKind] = &[CellKind::Node, CellKind::Node, CellKind::Number];
fn column(index: u32) -> ColumnIndex {
    ColumnIndex(index)
}

/// A descendant axis walk, or one of this direction, answering these right columns.
fn axis(direction: Direction, output: &[u32]) -> Operation {
    Operation::Axis {
        from: FROM,
        edges: FROM,
        domain: FROM,
        direction,
        id: column(0),
        parent: column(0),
        child: column(1),
        domain_id: column(0),
        domain_filter: Vec::new(),
        domain_view: Vec::new(),
        output: output
            .iter()
            .map(|at| JoinColumn {
                side: Side::Right,
                column: column(*at),
            })
            .collect(),
    }
}
/// An ordered match on this key of each side, ordered by this left column, answering this left column.
fn ordered(key: u32, left_order: u32, output: u32) -> Operation {
    Operation::OrderedMatch {
        left: FROM,
        right: FROM,
        left_on: vec![column(key)],
        right_on: vec![column(key)],
        left_order: column(left_order),
        right_order: column(1),
        output: vec![JoinColumn {
            side: Side::Left,
            column: column(output),
        }],
    }
}
/// An ordered depth-first walk with these root keys, to this destination column.
fn dfs(root_keys: &[u32], destination: u32) -> Operation {
    Operation::OrderedDfs {
        from: FROM,
        edges: FROM,
        root: column(0),
        source: column(0),
        destination: column(destination),
        order: column(2),
        edge: column(1),
        root_keys: root_keys.iter().map(|at| column(*at)).collect(),
        edge_keys: Vec::new(),
    }
}
/// The traversal over the three inputs.
fn over_three(operation: Operation, second: &'static [CellKind]) -> Traversal {
    Traversal {
        operation,
        second,
        inputs: 3,
    }
}

fn subject(traversal: Input) -> Result<Value, Failure> {
    let mut schemas = vec![
        InputShape {
            kinds: KINDS,
            tree: None,
        };
        traversal.inputs
    ];
    if let Some(second) = schemas.get_mut(1) {
        second.kinds = traversal.second;
    }
    TraversalShape::new(&schemas)
        .of(&traversal.operation, &[])
        .map(Cow::Owned)
}

/// Native views and explicit schema refusals do not depend on graph rows.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("ordered_match_preserves_projected_node_kind"),
        word: Word::Succeeds,
        input: || over_three(ordered(0, 1, 0), KINDS),
        expect: Expect::Value(Cow::Borrowed(&[CellKind::Node])),
    },
    Case {
        name: CaseName("ordered_dfs_checks_endpoint_kinds"),
        word: Word::Refuses,
        input: || over_three(dfs(&[], 1), KINDS),
        expect: Expect::Error(OperatingError::ShapeMismatch),
    },
    Case {
        name: CaseName("computed_axis_projection_remains_unsupported"),
        word: Word::Refuses,
        input: || over_three(axis(Direction::Descendant, &[0]), KINDS),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("computed_deep_axis_remains_unsupported"),
        word: Word::Refuses,
        input: || over_three(axis(Direction::Deep, &[]), KINDS),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("ordered_match_requires_numeric_order"),
        word: Word::Refuses,
        input: || over_three(ordered(0, 0, 0), KINDS),
        expect: Expect::Error(OperatingError::ShapeMismatch),
    },
    Case {
        name: CaseName("ordered_match_checks_output_columns"),
        word: Word::Refuses,
        input: || over_three(ordered(0, 1, 99), KINDS),
        expect: Expect::Error(OperatingError::ShapeMismatch),
    },
    Case {
        name: CaseName("ordered_match_checks_missing_keys_on_both_sides"),
        word: Word::Refuses,
        input: || over_three(ordered(99, 1, 0), KINDS),
        expect: Expect::Error(OperatingError::ShapeMismatch),
    },
    Case {
        name: CaseName("ordered_dfs_checks_root_key_count"),
        word: Word::Refuses,
        input: || over_three(dfs(&[0], 1), GRAPH),
        expect: Expect::Error(OperatingError::ShapeMismatch),
    },
    Case {
        name: CaseName("ordered_dfs_checks_endpoint_columns"),
        word: Word::Refuses,
        input: || over_three(dfs(&[], 99), GRAPH),
        expect: Expect::Error(OperatingError::Table(RowsError::NoSuchColumn { index: 99 })),
    },
    Case {
        name: CaseName("traversal_missing_input_remains_explicit"),
        word: Word::Refuses,
        input: || Traversal {
            operation: axis(Direction::Descendant, &[]),
            second: KINDS,
            inputs: 0,
        },
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

use ryo_values::domain::rows::errors::RowsError;
