//! Traversal shape refusals are pure representation-sensitive metadata contracts.
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
/// One native or invalid traversal contract.
#[derive(Clone, Copy)]
enum Scenario {
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

fn subject(scenario: Input) -> Result<Value, Failure> {
    let kinds = [CellKind::Node, CellKind::Number, CellKind::Number];
    let mut schemas = [InputShape {
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
            return TraversalShape::new(&[])
                .of(&operation, &[])
                .map(|kinds| kinds.len());
        }
    }
    TraversalShape::new(&schemas)
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
