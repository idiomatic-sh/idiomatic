//! Refusal and physical-route controls through the real compiled owner.
use super::super::fixtures::*;
use super::fixtures::ordered_strategy_fixture;
use super::fixtures::{MeaningCalls, run as run_file};
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::coverage::types::{Lacking, Partial};
use ryo_relations::domain::relations::{
    service::{kinds, node_id},
    types::{CellKind, FileId, Flag, Integer, Offset, Position, Relation, Span, Value as Cell},
};
use ryo_values::domain::stored_facts::types::PartialCause;
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input as PlanInput, StepId, StepScope},
    plans::types::{Operation, PlanStep},
    rows::types::{RowCount, Table},
};
use std::sync::Arc;
use std::sync::atomic::Ordering;
fn partial_positive_rows_survive_but_partial_absence_is_refused() {
    let relation = Relation::SemanticSyntax;
    let source = Table {
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
    };
    assert_eq!(
        source.cells.len(),
        source.kinds.len() * 3,
        "canonical provider shape"
    );
    let mut positive = plan(Scenario::ProjectRetry);
    positive.steps.truncate(1);
    positive.emit.step = StepId(0);
    positive.steps[0].operation = Operation::Select {
        from: PlanInput::Relation(relation),
        comparisons: vec![],
        kept: vec![ColumnIndex(0)],
    };
    let partials = [Partial {
        relation,
        cause: PartialCause("partial provider fixture".into()),
        lacking: Lacking::Any,
        at: ryo_execution_domain::domain::coverage::types::PartialPlace::Repository,
    }];
    let RootOutcome::Partial { table, affected } = run_file(
        positive,
        &[(relation, source.clone())],
        &partials,
        Arc::new(MeaningCalls::default()),
    ) else {
        panic!("usable positive partial rows")
    };
    assert_eq!(table.count, RowCount(3));
    assert_eq!(
        table.cells,
        [2, 1, 2].map(|at| Cell::Node(node_id(FileId(0), Position(at))))
    );
    assert_eq!(affected, vec![Some(FileId(0))]);
    let mut absent = plan(Scenario::ProjectRetry);
    absent.steps.truncate(1);
    absent.emit.step = StepId(0);
    absent.steps[0].operation = Operation::Difference {
        left: PlanInput::Relation(Relation::SyntaxTree),
        right: PlanInput::Relation(relation),
    };
    let calls = Arc::new(MeaningCalls::default());
    let RootOutcome::Partial { table, affected } = run_file(
        absent,
        &[(relation, source.clone()), (Relation::SyntaxTree, source)],
        &partials,
        Arc::clone(&calls),
    ) else {
        panic!("incomplete absence must remain partial")
    };
    assert_eq!(table.count, RowCount(0));
    assert_eq!(affected, vec![Some(FileId(0))]);
    assert_eq!(
        calls.sets.load(Ordering::Relaxed),
        0,
        "absence never enters row meaning on incomplete RHS"
    );
}
fn ordered_strategy_and_real_extra_consumer_decline_preserve_the_same_root() {
    let (request, schema) = ordered_strategy_fixture();
    let number = schema
        .iter()
        .position(|kind| *kind == CellKind::Number)
        .unwrap();
    let rows = [1, 3, 2].map(|number| {
        schema
            .iter()
            .map(|kind| match kind {
                CellKind::Node => Cell::Node(ryo_relations::domain::relations::types::NodeId(0)),
                CellKind::Symbol => Cell::Symbol(ryo_relations::domain::terms::types::Symbol(0)),
                CellKind::File => Cell::File(FileId(0)),
                CellKind::Span => Cell::Span(Span {
                    start: Offset(0),
                    end: Offset(0),
                }),
                CellKind::Number => Cell::Number(Integer(number)),
                CellKind::Flag => Cell::Flag(Flag::Raised),
            })
            .collect::<Vec<_>>()
    });
    let source = Table {
        kinds: schema.clone(),
        cells: rows.iter().flatten().copied().collect(),
        count: RowCount(3),
    };
    // The earlier internal-demand case observed this Project's two dominated
    // numbers. Make it an authored root to retain the assertion under the real protocol.
    let mut dominated = request.clone();
    dominated.emit.step = StepId(3);
    let RootOutcome::Complete(table) = run_file(
        dominated,
        &[(Relation::SourceLocation, source.clone())],
        &[],
        Arc::new(MeaningCalls::default()),
    ) else {
        panic!("dominated projection is complete")
    };
    assert_eq!(table.kinds, vec![CellKind::Number]);
    assert_eq!(
        table.cells,
        vec![Cell::Number(Integer(1)), Cell::Number(Integer(2))]
    );
    assert_eq!(table.count, RowCount(2));
    for downstream in [false, true] {
        for extra in [false, true] {
            let mut request = request.clone();
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
            let calls = Arc::new(MeaningCalls::default());
            let RootOutcome::Complete(table) = run_file(
                request,
                &[(Relation::SourceLocation, source.clone())],
                &[],
                Arc::clone(&calls),
            ) else {
                panic!("complete ordered root")
            };
            assert_eq!(table.kinds, schema);
            assert_eq!(table.count, RowCount(1));
            assert_eq!(table.cells, rows[1]);
            assert_eq!(table.cells[number], Cell::Number(Integer(3)));
            assert_eq!(
                calls.ordered.load(Ordering::Relaxed) > 0,
                !extra,
                "actual ordered meaning entered only for eligible strategy"
            );
            assert_eq!(
                calls.joins.load(Ordering::Relaxed) > 0,
                extra,
                "real extra reader selects ordinary Join"
            );
        }
    }
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Proof {
    PartialPositiveRowsSurviveButPartialAbsenceIsRefused,
    OrderedStrategyAndRealExtraConsumerDeclinePreserveTheSameRoot,
}

type Input = Proof;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("partial_positive_rows_survive_but_partial_absence_is_refused"),
        word: Word::Succeeds,
        input: || Proof::PartialPositiveRowsSurviveButPartialAbsenceIsRefused,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("ordered_strategy_and_real_extra_consumer_decline_preserve_the_same_root"),
        word: Word::Succeeds,
        input: || Proof::OrderedStrategyAndRealExtraConsumerDeclinePreserveTheSameRoot,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Proof::PartialPositiveRowsSurviveButPartialAbsenceIsRefused => {
            partial_positive_rows_survive_but_partial_absence_is_refused();
            Ok(())
        }
        Proof::OrderedStrategyAndRealExtraConsumerDeclinePreserveTheSameRoot => {
            ordered_strategy_and_real_extra_consumer_decline_preserve_the_same_root();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
