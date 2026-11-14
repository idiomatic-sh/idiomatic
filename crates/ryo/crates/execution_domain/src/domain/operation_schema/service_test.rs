//! Catalog coverage and virtual schema projection are pure domain contracts.
use std::borrow::Cow;

use super::{fixtures::*, service::schema_of, types::InputShape};
use crate::domain::{features::service::catalog, operating::errors::OperatingError};
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, HostedNumber, Input as Source, JoinColumn, Side},
    plans::{service::kind_of, types::Operation},
    run_metadata::types::Feature,
};
const FROM: Source = Source::Relation(Relation::Function);

/// What a row asks of the operation schemas.
enum Ask {
    /// How the declared operation kinds, the operator fixtures and the feature catalog cover each other.
    Coverage,
    /// The schema of an operation over inputs of these shapes.
    Columns(Operation, &'static [InputShape<'static>]),
}

/// How the declared kinds, the operator fixtures and the catalog's operation features cover each other.
#[derive(Debug, PartialEq)]
struct Coverage {
    /// How many kinds are declared, and how many distinct.
    declared: (usize, usize),
    /// How many fixtures there are, how many distinct kinds they have, and whether those are the declared kinds.
    fixtures: (usize, usize, bool),
    /// How many operation features the catalog has, and whether their kinds are the declared kinds.
    features: (usize, bool),
}

/// What the operation schemas answer.
#[derive(Debug, PartialEq)]
enum Answer {
    Coverage(Coverage),
    /// The kinds of the columns of an operation's answer.
    Kinds(Cow<'static, [CellKind]>),
}

type Input = Ask;
type Value = Answer;
type Failure = OperatingError;

fn coverage() -> Coverage {
    use std::collections::HashSet;
    let declared: HashSet<_> = OperationKind::ALL.iter().copied().collect();
    let fixtures: Vec<_> = operations().iter().map(kind_of).collect();
    let fixture_kinds: HashSet<_> = fixtures.iter().copied().collect();
    let features: Vec<_> = catalog()
        .into_iter()
        .filter_map(|feature| match feature {
            Feature::Operation(kind) => Some(kind),
            Feature::Family(_) | Feature::Direction(_) | Feature::Relation(_) => None,
        })
        .collect();
    Coverage {
        declared: (OperationKind::ALL.len(), declared.len()),
        fixtures: (
            fixtures.len(),
            fixture_kinds.len(),
            fixture_kinds == declared,
        ),
        features: (
            features.len(),
            features.iter().copied().collect::<HashSet<_>>() == declared,
        ),
    }
}

fn subject(ask: Input) -> Result<Value, Failure> {
    Ok(match ask {
        Ask::Coverage => Answer::Coverage(coverage()),
        Ask::Columns(operation, inputs) => {
            Answer::Kinds(Cow::Owned(schema_of(&operation, inputs, &[])?))
        }
    })
}

/// A demand join of `function` with itself through a reach lookup.
fn reach_join() -> Operation {
    use ryo_values::domain::plan_parts::types::ReachLookup;
    let right = |column| JoinColumn {
        side: Side::Right,
        column: ColumnIndex(column),
    };
    Operation::Join {
        left: FROM,
        right: FROM,
        left_on: vec![ColumnIndex(0)],
        right_on: vec![ColumnIndex(2)],
        output: vec![right(0), right(1), right(0)],
        filter: vec![],
        view: vec![],
        reach: Some(ReachLookup {
            source: ColumnIndex(1),
            destination: ColumnIndex(0),
            keys: vec![ColumnIndex(2)],
            columns: vec![ColumnIndex(2), ColumnIndex(0), ColumnIndex(1)],
        }),
    }
}

/// Every kind a cell can be.
const SUPPLIED: &[CellKind] = &[
    CellKind::Number,
    CellKind::Symbol,
    CellKind::Node,
    CellKind::Span,
    CellKind::Flag,
    CellKind::File,
];

/// Edges whose source is a node and whose destination is a file.
const NODE_TO_FILE: &[InputShape<'static>] = &[InputShape {
    kinds: &[CellKind::Node, CellKind::File],
    tree: None,
}];

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_declared_kind_has_one_evaluator_fixture_and_one_feature"),
        word: Word::Succeeds,
        input: || Ask::Coverage,
        expect: Expect::Value(Answer::Coverage(Coverage {
            declared: (26, 26),
            fixtures: (26, 26, true),
            features: (26, true),
        })),
    },
    Case {
        name: CaseName(
            "a_demand_join_schema_uses_the_virtual_partition_source_destination_projection",
        ),
        word: Word::Succeeds,
        input: || {
            Ask::Columns(
                reach_join(),
                &[
                    InputShape {
                        kinds: &[CellKind::Node],
                        tree: None,
                    },
                    InputShape {
                        kinds: &[CellKind::Symbol, CellKind::Node, CellKind::File],
                        tree: None,
                    },
                ],
            )
        },
        expect: Expect::Value(Answer::Kinds(Cow::Borrowed(&[
            CellKind::Symbol,
            CellKind::File,
            CellKind::Symbol,
        ]))),
    },
    Case {
        name: CaseName("hosted_metadata_preserves_every_supplied_kind_without_constructing_rows"),
        word: Word::Succeeds,
        input: || {
            Ask::Columns(
                Operation::Hosted {
                    relation: HostedNumber(0),
                },
                &[InputShape {
                    kinds: SUPPLIED,
                    tree: None,
                }],
            )
        },
        expect: Expect::Value(Answer::Kinds(Cow::Borrowed(SUPPLIED))),
    },
    Case {
        name: CaseName("hosted_without_its_request_source_is_missing_not_unsupported"),
        word: Word::Refuses,
        input: || {
            Ask::Columns(
                Operation::Hosted {
                    relation: HostedNumber(0),
                },
                &[],
            )
        },
        expect: Expect::Error(OperatingError::MissingInput { position: 1 }),
    },
    Case {
        name: CaseName("cycles_whose_source_and_destination_differ_in_kind_is_refused"),
        word: Word::Refuses,
        input: || {
            Ask::Columns(
                Operation::Cycles {
                    edges: FROM,
                    source: ColumnIndex(0),
                    destination: ColumnIndex(1),
                },
                NODE_TO_FILE,
            )
        },
        expect: Expect::Error(OperatingError::ShapeMismatch),
    },
    Case {
        name: CaseName("a_closure_whose_source_and_destination_differ_in_kind_is_refused"),
        word: Word::Refuses,
        input: || {
            Ask::Columns(
                Operation::Closure {
                    edges: FROM,
                    keys: vec![],
                    source: ColumnIndex(0),
                    destination: ColumnIndex(1),
                },
                NODE_TO_FILE,
            )
        },
        expect: Expect::Error(OperatingError::ShapeMismatch),
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
mod join;
