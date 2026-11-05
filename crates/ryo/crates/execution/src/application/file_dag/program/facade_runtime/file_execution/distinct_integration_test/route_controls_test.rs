//! Refusal and physical-route controls through the real compiled owner.
use super::*;
use crate::domain::coverage::types::{Lacking, Partial};
use ryo_operators::inbound::use_cases::operators::{
    interface::{OperatorAnswer, OperatorCall, OperatorsUseCase, PortableOperatorFault},
    use_case::OperatorsUseCaseImpl,
};
use ryo_relations::domain::relations::{
    service::to_bits,
    types::{ColumnsView, FactColumn},
};
use ryo_values::domain::plan_parts::types::{Comparison, Operand, Test};
use ryo_values::domain::stored_facts::types::PartialCause;
use std::sync::atomic::{AtomicUsize, Ordering};
#[derive(Debug, Default)]
struct MeaningCalls {
    ordered: AtomicUsize,
    joins: AtomicUsize,
    sets: AtomicUsize,
}
impl OperatorsUseCase for MeaningCalls {
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        match &call {
            OperatorCall::OrderedExistence { .. } => {
                self.ordered.fetch_add(1, Ordering::Relaxed);
            }
            OperatorCall::SetRow { .. } => {
                self.sets.fetch_add(1, Ordering::Relaxed);
            }
            OperatorCall::EqualityJoin { .. } => {
                self.joins.fetch_add(1, Ordering::Relaxed);
            }
            _ => {}
        }
        OperatorsUseCaseImpl::new().operators(call)
    }
}
fn run(
    plan: Plan,
    providers: &[(Relation, Table)],
    partials: &[Partial],
    calls: Arc<MeaningCalls>,
) -> RootOutcome {
    let derived = crate::application::file_dag::testing::program_derived_port();
    let prepared = facade::prepare(
        Compiled {
            plans: vec![plan],
            bindings: vec![vec![]],
            schedule: RuleSchedule::Independent,
            observations: vec![vec![]],
        },
        &[rule()],
        &[Digest([0; 16])],
        &[],
        Arc::clone(&derived),
        ObservationIdentity::Independent,
    )
    .unwrap();
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(calls)),
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("route-control.py".into()),
            size: FileSize(0),
            modified: Nanoseconds(0),
        })],
    ));
    let mut symbols = Symbols::default();
    let mut file = open_file(
        &prepared,
        FileId(0),
        ScopePartials::File {
            evaluation: partials,
            publication: partials,
        },
        &[true],
        services,
        b"",
        true,
        &mut symbols,
        None,
        None,
    )
    .unwrap();
    let stored = providers
        .iter()
        .map(|(_, table)| {
            (0..table.kinds.len())
                .map(|column| {
                    FactColumn(
                        table
                            .cells
                            .chunks_exact(table.kinds.len())
                            .map(|row| to_bits(row[column]).0)
                            .collect(),
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let read = |relation| {
        providers
            .iter()
            .position(|(kind, _)| *kind == relation)
            .map(|at| {
                Rows::Stored(ColumnsView {
                    kinds: kinds(relation),
                    columns: &stored[at],
                })
            })
            .ok_or(OperatingError::NotRead)
    };
    let outcome = file
        .rule(
            RuleIndex(0),
            RuleDemand {
                emit: EmitDemand::Enabled,
                observations: ObservationDemand::NotRequired,
            },
            &read,
            &mut symbols,
        )
        .unwrap();
    file.finish().unwrap();
    outcome.emit
}
#[test]
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
        at: None,
    }];
    let RootOutcome::Partial { table, affected } = run(
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
    let RootOutcome::Partial { table, affected } = run(
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
#[test]
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
    let RootOutcome::Complete(table) = run(
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
            let RootOutcome::Complete(table) = run(
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
fn ordered_strategy_fixture() -> (Plan, Vec<CellKind>) {
    use ryo_values::domain::plan_parts::types::{JoinColumn, Side};
    let schema = kinds(ryo_relations::domain::relations::types::Relation::SourceLocation).to_vec();
    let number = ColumnIndex(
        schema
            .iter()
            .position(|kind| *kind == CellKind::Number)
            .expect("typed numeric source") as u32,
    );
    let source = Operation::Select {
        from: PlanInput::Relation(
            ryo_relations::domain::relations::types::Relation::SourceLocation,
        ),
        comparisons: vec![],
        kept: vec![],
    };
    let join = Operation::Join {
        left: PlanInput::Step(StepId(0)),
        right: PlanInput::Step(StepId(0)),
        left_on: vec![ColumnIndex(0)],
        right_on: vec![ColumnIndex(0)],
        output: vec![
            JoinColumn {
                side: Side::Left,
                column: number,
            },
            JoinColumn {
                side: Side::Right,
                column: number,
            },
        ],
        filter: vec![],
        view: vec![],
        reach: None,
    };
    let select = Operation::Select {
        from: PlanInput::Step(StepId(1)),
        comparisons: vec![Comparison {
            test: Test::LessThan,
            left: Operand::Column(ColumnIndex(0)),
            right: Operand::Column(ColumnIndex(1)),
            origin: None,
        }],
        kept: vec![],
    };
    let project = Operation::Project {
        from: PlanInput::Step(StepId(2)),
        columns: vec![ColumnIndex(0)],
        distinct: RepeatedRows::Dropped,
    };
    let anti = Operation::Antijoin {
        left: PlanInput::Step(StepId(0)),
        right: PlanInput::Step(StepId(3)),
        left_on: vec![number],
        right_on: vec![ColumnIndex(0)],
    };
    (
        Plan {
            observation_inputs: Vec::new(),
            observations: Vec::new(),
            parameters: vec![],
            steps: [source, join, select, project, anti]
                .into_iter()
                .map(|operation| PlanStep {
                    operation,
                    scope: StepScope::PerFile,
                })
                .collect(),
            comparison_origins: vec![],
            emit: Emit {
                step: StepId(4),
                subject: ColumnIndex(0),
                key: vec![],
                headings: vec![],
            },
            frontier: vec![],
        },
        schema,
    )
}
