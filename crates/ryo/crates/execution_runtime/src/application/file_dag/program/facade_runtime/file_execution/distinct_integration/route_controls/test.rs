//! Refusal and physical-route controls through the real compiled owner.
use super::super::*;
use crate::outbound::operators::interface::OperatorsPort as _;
use ryo_execution_domain::domain::coverage::types::{Lacking, Partial};
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase;
use ryo_operators::inbound::use_cases::operators::types::OperatorAnswer;
use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
use ryo_relations::domain::relations::{
    service::to_bits,
    types::{ColumnsView, FactColumn},
};
use ryo_values::domain::plan_parts::types::{Operand, PlanComparison, Test};
use ryo_values::domain::stored_facts::types::PartialCause;
use std::sync::atomic::{AtomicUsize, Ordering};
#[derive(Debug, Default)]
struct MeaningCalls {
    ordered: AtomicUsize,
    joins: AtomicUsize,
    sets: AtomicUsize,
}
impl crate::outbound::operators::interface::OperatorsPort for MeaningCalls {
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        match &call {
            OperatorCall::OrderedExistence { .. } => {
                self.ordered.fetch_add(1, Ordering::Relaxed);
            }
            OperatorCall::Evaluate {
                operation:
                    ryo_values::domain::plans::types::Operation::Union { .. }
                    | ryo_values::domain::plans::types::Operation::Intersection { .. }
                    | ryo_values::domain::plans::types::Operation::Difference { .. }
                    | ryo_values::domain::plans::types::Operation::Antijoin { .. },
                ..
            } => {
                self.sets.fetch_add(1, Ordering::Relaxed);
            }
            OperatorCall::Evaluate {
                operation: ryo_values::domain::plans::types::Operation::Join { .. },
                ..
            } => {
                self.joins.fetch_add(1, Ordering::Relaxed);
            }
            _ => {}
        }
        crate::application::file_dag::fixtures::operators().operators(call)
    }
}
fn run(
    plan: Plan,
    providers: &[(Relation, Table)],
    partials: &[Partial],
    calls: Arc<MeaningCalls>,
) -> RootOutcome {
    run_read(
        plan,
        providers,
        (partials, partials),
        Supplied::Stored,
        calls,
    )
}
/// How a provider hands its rows over: stored columns, or a computed table.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Supplied {
    Stored,
    Computed,
}
/// What the rule of `plan` answers when its evaluation sees the partial reads `evaluation` and its publication `publication`.
fn run_read(
    plan: Plan,
    providers: &[(Relation, Table)],
    (evaluation, publication): (&[Partial], &[Partial]),
    supplied: Supplied,
    calls: Arc<MeaningCalls>,
) -> RootOutcome {
    let derived = ryo_execution_store::application::file_dag::fixtures::program_derived_port();
    let prepared = crate::application::file_dag::program::preparation::service::prepare(
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
            evaluation,
            publication,
        },
        &[FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        }],
        services,
        ryo_values::domain::files::types::SourceView(b""),
        ReuseEligibility::Eligible,
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
                if supplied == Supplied::Computed {
                    return Rows::Computed(&providers[at].1);
                }
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
        at: ryo_execution_domain::domain::coverage::types::PartialPlace::Repository,
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
/// The first location of each line, by byte: an antijoin of the locations over the self join of each line's locations. With `kept`
/// empty the engine recognises it as an ordered existence; keeping `[0, 1]`, the same meaning, declines the shape.
fn first_of_each_line(kept: Vec<ColumnIndex>) -> Plan {
    use ryo_values::domain::plan_parts::types::{ComparisonOrigin, JoinColumn, RepeatedRows, Side};
    let (line, byte) = (ColumnIndex(3), ColumnIndex(5));
    let mut plan = ordered_strategy_fixture().0;
    plan.steps = [
        Operation::Select {
            from: PlanInput::Relation(Relation::SourceLocation),
            comparisons: vec![],
            kept: vec![],
        },
        Operation::Join {
            left: PlanInput::Step(StepId(0)),
            right: PlanInput::Step(StepId(0)),
            left_on: vec![line],
            right_on: vec![line],
            output: vec![
                JoinColumn {
                    side: Side::Left,
                    column: byte,
                },
                JoinColumn {
                    side: Side::Right,
                    column: byte,
                },
            ],
            filter: vec![],
            view: vec![],
            reach: None,
        },
        Operation::Select {
            from: PlanInput::Step(StepId(1)),
            kept,
            comparisons: vec![PlanComparison {
                test: Test::LessThan,
                left: Operand::Column(ColumnIndex(0)),
                right: Operand::Column(ColumnIndex(1)),
                origin: ComparisonOrigin::Unrecorded,
            }],
        },
        Operation::Project {
            from: PlanInput::Step(StepId(2)),
            columns: vec![ColumnIndex(0)],
            distinct: RepeatedRows::Dropped,
        },
        Operation::Antijoin {
            left: PlanInput::Step(StepId(0)),
            right: PlanInput::Step(StepId(3)),
            left_on: vec![byte],
            right_on: vec![ColumnIndex(0)],
        },
    ]
    .into_iter()
    .map(|operation| PlanStep {
        scope: StepScope::PerFile,
        operation,
    })
    .collect();
    plan.emit.step = StepId(4);
    plan
}
/// A recognised ordered existence over a computed source answers what its declined twin answers, whole and read in part: read in
/// part, it is partial in the file and in the repository.
#[test]
fn a_recognised_ordered_existence_read_in_part_answers_as_its_declined_twin() {
    let schema = kinds(Relation::SourceLocation).to_vec();
    // Two locations of file 0 on the same line, at bytes 5 and 1.
    let cells = [(1, 5), (2, 1)]
        .into_iter()
        .flat_map(|(row, byte)| {
            schema
                .iter()
                .enumerate()
                .map(move |(column, kind)| match kind {
                    CellKind::Node => Cell::Node(node_id(FileId(0), Position(row))),
                    CellKind::File => Cell::File(FileId(0)),
                    CellKind::Span => Cell::Span(Span {
                        start: Offset(0),
                        end: Offset(0),
                    }),
                    CellKind::Number => Cell::Number(Integer(if column == 5 { byte } else { 1 })),
                    CellKind::Symbol => {
                        Cell::Symbol(ryo_relations::domain::terms::types::Symbol(0))
                    }
                    CellKind::Flag => Cell::Flag(Flag::Lowered),
                })
        })
        .collect();
    let source = Table {
        kinds: schema,
        cells,
        count: RowCount(2),
    };
    let partials = [Partial {
        relation: Relation::SourceLocation,
        cause: PartialCause("partial provider fixture".into()),
        lacking: Lacking::Any,
        at: ryo_execution_domain::domain::coverage::types::PartialPlace::File(FileId(0)),
    }];
    let answer = |kept: Vec<ColumnIndex>, publication: &[Partial]| match run_read(
        first_of_each_line(kept),
        &[(Relation::SourceLocation, source.clone())],
        (&[], publication),
        Supplied::Computed,
        Arc::new(MeaningCalls::default()),
    ) {
        RootOutcome::Partial { table, affected } => format!("partial {table:?} {affected:?}"),
        RootOutcome::Complete(table) => format!("complete {table:?}"),
        RootOutcome::Failed(error) => format!("failed {error:?}"),
        RootOutcome::Unknown(error) => format!("unknown {error:?}"),
        _ => "other".to_owned(),
    };
    for publication in [&[][..], &partials[..]] {
        assert_eq!(
            answer(vec![], publication),
            answer(vec![ColumnIndex(0), ColumnIndex(1)], publication),
            "read in part: {}",
            !publication.is_empty()
        );
    }
    let read_in_part = answer(vec![], &partials);
    assert!(read_in_part.starts_with("partial"), "{read_in_part}");
    assert!(
        read_in_part.ends_with("[None, Some(FileId(0))]"),
        "{read_in_part}"
    );
}
pub(crate) fn ordered_strategy_fixture() -> (Plan, Vec<CellKind>) {
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
        comparisons: vec![PlanComparison {
            test: Test::LessThan,
            left: Operand::Column(ColumnIndex(0)),
            right: Operand::Column(ColumnIndex(1)),
            origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
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
