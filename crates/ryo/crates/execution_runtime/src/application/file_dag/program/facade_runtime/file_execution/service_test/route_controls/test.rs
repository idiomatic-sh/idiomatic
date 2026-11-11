//! Refusal and physical-route controls through the real compiled owner.
use super::super::fixtures::{PlanShape, Root, plan};
use super::fixtures::ordered_strategy_fixture;
use super::fixtures::{MeaningCalls, run as run_file};
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::coverage::types::{Lacking, Partial};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::{
    service::{kinds, node_id},
    types::{
        CellKind, FileId, Flag, Integer, NodeId, Offset, Position, Relation, Span, Value as Cell,
    },
};
use ryo_values::domain::stored_facts::types::PartialCause;
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input as PlanInput, StepId, StepScope},
    plans::types::{Operation, PlanStep},
    rows::types::{RowCount, Table},
};
use std::borrow::Cow;
use std::sync::Arc;
use std::sync::atomic::Ordering;

/// The plan a row routes through one file.
#[derive(Clone, Copy, Debug)]
enum Route {
    /// Selects the partial relation's rows, which are positive.
    PartialPositive,
    /// Subtracts the partial relation from the complete syntax tree: an absence over it.
    PartialAbsence,
    /// The ordered strategy fixture's dominated projection, as an authored root.
    Dominated,
    /// The ordered strategy fixture's root, with an extra consumer of its projection and a
    /// downstream extension, or without them.
    Ordered { downstream: bool, extra: bool },
}

/// What the routed root answered: its cell kinds, its outcome, the files a partial root names,
/// and the set, ordered-existence and join meanings it entered.
#[derive(Debug, PartialEq)]
struct Routed {
    kinds: Cow<'static, [CellKind]>,
    root: Root,
    affected: Cow<'static, [Option<FileId>]>,
    sets: usize,
    ordered: usize,
    joins: usize,
}

type Input = Route;
type Value = Routed;
type Failure = OperatingError;

const fn routed(
    kinds: &'static [CellKind],
    root: Root,
    affected: &'static [Option<FileId>],
    (sets, ordered, joins): (usize, usize, usize),
) -> Routed {
    Routed {
        kinds: Cow::Borrowed(kinds),
        root,
        affected: Cow::Borrowed(affected),
        sets,
        ordered,
        joins,
    }
}

/// The node at this position of file 0, whose id is its position.
const fn node(at: u64) -> Cell {
    Cell::Node(NodeId(at))
}

/// The ordered fixture's schema: a node, its file and span, and six numbers.
const SCHEMA: &[CellKind] = &[
    CellKind::Node,
    CellKind::File,
    CellKind::Span,
    CellKind::Number,
    CellKind::Number,
    CellKind::Number,
    CellKind::Number,
    CellKind::Number,
    CellKind::Number,
];

/// The ordered root's one row, the source row whose numbers are 3, reached through these set,
/// ordered-existence and join meanings.
const fn ordered(calls: (usize, usize, usize)) -> Routed {
    routed(
        SCHEMA,
        Root::complete(
            &[
                Cell::Node(NodeId(0)),
                Cell::File(FileId(0)),
                Cell::Span(Span {
                    start: Offset(0),
                    end: Offset(0),
                }),
                Cell::Number(Integer(3)),
                Cell::Number(Integer(3)),
                Cell::Number(Integer(3)),
                Cell::Number(Integer(3)),
                Cell::Number(Integer(3)),
                Cell::Number(Integer(3)),
            ],
            1,
        ),
        &[],
        calls,
    )
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("partial_positive_rows_survive_but_partial_absence_is_refused"),
        word: Word::Succeeds,
        input: || Route::PartialPositive,
        expect: Expect::Value(routed(
            &[CellKind::Node],
            Root::Partial {
                cells: Cow::Borrowed(&[node(2), node(1), node(2)]),
                rows: 3,
            },
            &[Some(FileId(0))],
            (0, 0, 0),
        )),
    },
    Case {
        name: CaseName(
            "an_absence_over_a_partial_relation_stays_partial_without_entering_set_meaning",
        ),
        word: Word::Succeeds,
        input: || Route::PartialAbsence,
        expect: Expect::Value(routed(
            &[
                CellKind::Node,
                CellKind::Symbol,
                CellKind::Symbol,
                CellKind::File,
                CellKind::Span,
            ],
            Root::Partial {
                cells: Cow::Borrowed(&[]),
                rows: 0,
            },
            &[Some(FileId(0))],
            (0, 0, 0),
        )),
    },
    Case {
        name: CaseName("the_ordered_fixture_s_dominated_projection_keeps_its_two_numbers"),
        word: Word::Succeeds,
        input: || Route::Dominated,
        expect: Expect::Value(routed(
            &[CellKind::Number],
            Root::complete(&[Cell::Number(Integer(1)), Cell::Number(Integer(2))], 2),
            &[],
            (0, 0, 3),
        )),
    },
    Case {
        name: CaseName("ordered_strategy_and_real_extra_consumer_decline_preserve_the_same_root"),
        word: Word::Succeeds,
        input: || Route::Ordered {
            downstream: false,
            extra: false,
        },
        expect: Expect::Value(ordered((3, 1, 0))),
    },
    Case {
        name: CaseName("an_extra_consumer_declines_the_ordered_strategy_for_the_same_root"),
        word: Word::Succeeds,
        input: || Route::Ordered {
            downstream: false,
            extra: true,
        },
        expect: Expect::Value(ordered((3, 0, 3))),
    },
    Case {
        name: CaseName("a_downstream_extension_keeps_the_ordered_strategy_and_the_same_root"),
        word: Word::Succeeds,
        input: || Route::Ordered {
            downstream: true,
            extra: false,
        },
        expect: Expect::Value(ordered((3, 1, 0))),
    },
    Case {
        name: CaseName("an_extra_consumer_and_a_downstream_extension_decline_the_ordered_strategy"),
        word: Word::Succeeds,
        input: || Route::Ordered {
            downstream: true,
            extra: true,
        },
        expect: Expect::Value(ordered((3, 0, 3))),
    },
];

/// Three semantic syntax rows at positions 2, 1 and 2 of file 0.
fn semantic_rows() -> Table {
    let relation = Relation::SemanticSyntax;
    Table {
        kinds: kinds(relation).to_vec(),
        count: RowCount(3),
        cells: [2, 1, 2]
            .into_iter()
            .flat_map(|at| {
                kinds(relation).iter().map(move |kind| match kind {
                    CellKind::Node => Cell::Node(node_id(FileId(0), Position(at))),
                    CellKind::Symbol => {
                        Cell::Symbol(ryo_relations::domain::terms::types::Symbol(0))
                    }
                    CellKind::File => Cell::File(FileId(0)),
                    CellKind::Span => Cell::Span(Span {
                        start: Offset(0),
                        end: Offset(0),
                    }),
                    _ => panic!("canonical tree schema"),
                })
            })
            .collect(),
    }
}

/// The ordered fixture's source: three rows whose numbers are 1, 3 and 2.
fn ordered_rows(schema: &[CellKind]) -> Table {
    let cells = [1, 3, 2]
        .into_iter()
        .flat_map(|number| {
            schema.iter().map(move |kind| match kind {
                CellKind::Node => Cell::Node(NodeId(0)),
                CellKind::Symbol => Cell::Symbol(ryo_relations::domain::terms::types::Symbol(0)),
                CellKind::File => Cell::File(FileId(0)),
                CellKind::Span => Cell::Span(Span {
                    start: Offset(0),
                    end: Offset(0),
                }),
                CellKind::Number => Cell::Number(Integer(number)),
                CellKind::Flag => Cell::Flag(Flag::Raised),
            })
        })
        .collect();
    Table {
        kinds: schema.to_vec(),
        cells,
        count: RowCount(3),
    }
}

/// Builds the route's plan and sources, runs it through one file and reads what it answered.
fn subject(route: Input) -> Result<Value, Failure> {
    let relation = Relation::SemanticSyntax;
    let partials = [Partial {
        relation,
        cause: PartialCause("partial provider fixture".into()),
        lacking: Lacking::Any,
        at: ryo_execution_domain::domain::coverage::types::PartialPlace::Repository,
    }];
    let calls = Arc::new(MeaningCalls::default());
    let outcome = match route {
        Route::PartialPositive | Route::PartialAbsence => {
            let mut request = plan(PlanShape::ProjectRetry);
            request.steps.truncate(1);
            request.emit.step = StepId(0);
            request.steps[0].operation = if matches!(route, Route::PartialPositive) {
                Operation::Select {
                    from: PlanInput::Relation(relation),
                    comparisons: vec![],
                    kept: vec![ColumnIndex(0)],
                }
            } else {
                Operation::Difference {
                    left: PlanInput::Relation(Relation::SyntaxTree),
                    right: PlanInput::Relation(relation),
                }
            };
            let sources = if matches!(route, Route::PartialPositive) {
                vec![(relation, semantic_rows())]
            } else {
                vec![
                    (relation, semantic_rows()),
                    (Relation::SyntaxTree, semantic_rows()),
                ]
            };
            run_file(request, &sources, &partials, Arc::clone(&calls))
        }
        Route::Dominated | Route::Ordered { .. } => {
            let (mut request, schema) = ordered_strategy_fixture();
            match route {
                Route::Ordered { downstream, extra } => {
                    if extra {
                        request.frontier.push(PlanInput::Step(StepId(3)));
                    }
                    if downstream {
                        request.steps.push(PlanStep {
                            scope: StepScope::PerFile,
                            operation: Operation::Extend {
                                from: PlanInput::Step(StepId(4)),
                                values: vec![],
                            },
                        });
                        request.emit.step = StepId(5);
                    }
                }
                // The earlier internal-demand case observed this Project's two dominated
                // numbers; as an authored root it is observed under the real protocol.
                _ => request.emit.step = StepId(3),
            }
            run_file(
                request,
                &[(Relation::SourceLocation, ordered_rows(&schema))],
                &[],
                Arc::clone(&calls),
            )
        }
    };
    let (kinds, affected) = match &outcome {
        RootOutcome::Complete(table) => (table.kinds.clone(), vec![]),
        RootOutcome::Partial { table, affected } => (table.kinds.clone(), affected.clone()),
        _ => (vec![], vec![]),
    };
    Ok(Routed {
        kinds: Cow::Owned(kinds),
        root: Root::from(outcome),
        affected: Cow::Owned(affected),
        sets: calls.sets.load(Ordering::Relaxed),
        ordered: calls.ordered.load(Ordering::Relaxed),
        joins: calls.joins.load(Ordering::Relaxed),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
