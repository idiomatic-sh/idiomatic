//! Original scan/transport/key assertions through pure scanning or real compiled execution.
use super::super::*;
use ryo_execution_domain::domain::{
    replay::{service::scan, types::Collect},
    table_scratch::types::OperatorScratch,
};
use ryo_values::domain::{
    hosted::types::{HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow},
    plan_parts::types::HostedNumber,
    row_texts::types::Cell as HostedCell,
    rows::errors::RowsError,
};
const SOURCE: &[u8] = b"replayed";
fn files(count: u32) -> Table {
    Table {
        kinds: vec![CellKind::File],
        cells: (0..count).map(|file| Cell::File(FileId(file))).collect(),
        count: RowCount(count as usize),
    }
}
fn context<T>(run: impl FnOnce(&mut OperatorContext<'_>) -> T) -> T {
    let mut symbols = Symbols::default();
    run(&mut OperatorContext {
        phase: ryo_execution_domain::domain::operating::types::AfterTheFiles::Before,
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        matcher: ryo_execution_domain::domain::matching::types::PatternMatcher(
            &ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl,
        ),
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: ryo_values::domain::files::types::SourceView(SOURCE),
        paths: &[],
        arenas: &[],
        memo: &mut vec![],
        scratch: &mut OperatorScratch::default(),
    })
}
#[test]
fn a_scan_passes_on_every_row_of_a_table() {
    context(|context| {
        let from = files(3);
        let mut into = Table::default();
        scan(
            &Rows::Computed(&from),
            &mut vec![],
            context,
            &mut Collect(&mut into),
        )
        .unwrap();
        assert_eq!(into.cells, from.cells);
    });
}
struct CheckedSink {
    calls: usize,
    refuse: bool,
    rows: Vec<Cell>,
}
impl RowSink for CheckedSink {
    fn row(
        &mut self,
        context: &mut OperatorContext<'_>,
        row: &[Cell],
    ) -> Result<(), OperatingError> {
        assert_eq!(
            context.source.0, SOURCE,
            "the actual engine sink borrows the original source context"
        );
        self.calls += 1;
        if self.refuse {
            return Err(OperatingError::NotRun);
        }
        self.rows.extend_from_slice(row);
        Ok(())
    }
}
#[test]
fn a_scan_stops_at_the_row_the_taker_refuses() {
    context(|context| {
        let mut sink = CheckedSink {
            calls: 0,
            refuse: true,
            rows: vec![],
        };
        assert_eq!(
            scan(&Rows::Computed(&files(3)), &mut vec![], context, &mut sink),
            Err(OperatingError::NotRun)
        );
        assert_eq!(sink.calls, 1);
    });
}
fn authored(steps: Vec<PlanStep>) -> Plan {
    Plan {
        observation_inputs: vec![],
        observations: vec![],
        parameters: vec![],
        emit: Emit {
            step: StepId((steps.len() - 1) as u32),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        steps,
        frontier: vec![],
        comparison_origins: vec![],
    }
}
fn projected(column: u32) -> Plan {
    authored(vec![
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: PlanInput::Relation(Relation::LanguageTarget),
                comparisons: vec![],
                kept: vec![ColumnIndex(0)],
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Project {
                from: PlanInput::Step(StepId(0)),
                columns: vec![ColumnIndex(column)],
                distinct: RepeatedRows::Kept,
            },
        },
    ])
}
fn project_into(column: u32, sink: &mut dyn RowSink) -> Result<(), OperatingError> {
    let derived = ryo_execution_store::application::file_dag::fixtures::program_derived_port();
    let prepared = crate::application::file_dag::program::preparation::service::prepare(
        Compiled {
            schedule: RuleSchedule::Independent,
            observations: vec![vec![]],
            plans: vec![projected(column)],
            bindings: vec![vec![]],
        },
        &[rule()],
        &[Digest([0; 16])],
        &[],
        Arc::clone(&derived),
        ObservationIdentity::Independent,
    )?;
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(std::sync::Arc::new(
            crate::application::file_dag::fixtures::operators(),
        ))),
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("replayed.py".into()),
            size: FileSize(SOURCE.len() as u64),
            modified: Nanoseconds(0),
        })],
    ));
    let mut symbols = Symbols::default();
    let mut file = open_file(
        &prepared,
        FileId(0),
        ScopePartials::File {
            evaluation: &[],
            publication: &[],
        },
        &[FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        }],
        services,
        ryo_values::domain::files::types::SourceView(SOURCE),
        ReuseEligibility::Eligible,
        &mut symbols,
        None,
        None,
    )?;
    let table = Table {
        kinds: kinds(Relation::LanguageTarget).to_vec(),
        cells: vec![
            Cell::File(FileId(0)),
            Cell::Number(Integer(3)),
            Cell::Number(Integer(10)),
            Cell::File(FileId(1)),
            Cell::Number(Integer(3)),
            Cell::Number(Integer(10)),
        ],
        count: RowCount(2),
    };
    let read = |relation| {
        if relation == Relation::LanguageTarget {
            Ok(Rows::Computed(&table))
        } else {
            Err(OperatingError::NotRead)
        }
    };
    let result = forward(&mut file, &read, &mut symbols, sink).map(|_| ());
    let finish = file.finish().map(|_| ());
    result.and(finish)
}
#[test]
fn the_engine_sink_reads_the_original_source_context() {
    let mut sink = CheckedSink {
        calls: 0,
        refuse: false,
        rows: vec![],
    };
    project_into(0, &mut sink).unwrap();
    assert_eq!(sink.rows, files(2).cells);
    assert_eq!(sink.calls, 2);
}
#[test]
fn a_real_step_passes_on_the_rows_it_computes() {
    let mut into = CheckedSink {
        calls: 0,
        refuse: false,
        rows: vec![],
    };
    project_into(0, &mut into).unwrap();
    assert_eq!(into.rows, files(2).cells);
}
#[test]
fn rows_the_real_computation_refuses_are_not_passed_on() {
    let mut sink = CheckedSink {
        calls: 0,
        refuse: false,
        rows: vec![],
    };
    assert!(matches!(
        project_into(99, &mut sink),
        Err(OperatingError::Table(RowsError::NoSuchColumn { index: 99 }))
    ));
    assert_eq!(sink.calls, 0);
}
fn hosted(name: &str, width: usize, rows: &[&[i64]]) -> HostRelation {
    HostRelation {
        name: HostName(name.into()),
        columns: (0..width)
            .map(|at| HostColumn {
                name: HostColumnName(format!("value-{at}").into()),
                kind: HostKind::Number,
            })
            .collect(),
        rows: rows
            .iter()
            .map(|row| {
                HostRow(
                    row.iter()
                        .map(|number| HostedCell::Number(Integer(*number)))
                        .collect(),
                )
            })
            .collect(),
    }
}
fn indexed_join(key: i64, column: u32) -> Result<Table, OperatingError> {
    let steps = vec![
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Hosted {
                relation: HostedNumber(0),
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Hosted {
                relation: HostedNumber(1),
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Join {
                left: PlanInput::Step(StepId(0)),
                right: PlanInput::Step(StepId(1)),
                left_on: vec![ColumnIndex(column)],
                right_on: vec![ColumnIndex(0)],
                output: vec![JoinColumn {
                    side: Side::Right,
                    column: ColumnIndex(1),
                }],
                filter: vec![],
                view: vec![],
                reach: None,
            },
        },
    ];
    let derived = ryo_execution_store::application::file_dag::fixtures::program_derived_port();
    let hosts = [
        hosted("left", 1, &[&[key]]),
        hosted("right", 2, &[&[1, 10], &[2, 20], &[1, 30]]),
    ];
    let prepared = crate::application::file_dag::program::preparation::service::prepare(
        Compiled {
            schedule: RuleSchedule::Independent,
            observations: vec![vec![]],
            plans: vec![authored(steps)],
            bindings: vec![vec![]],
        },
        &[rule()],
        &[Digest([0; 16])],
        &hosts,
        Arc::clone(&derived),
        ObservationIdentity::Independent,
    )?;
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(std::sync::Arc::new(
            crate::application::file_dag::fixtures::operators(),
        ))),
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("indexed-join.py".into()),
            size: FileSize(0),
            modified: Nanoseconds(0),
        })],
    ));
    let mut symbols = Symbols::default();
    let mut file = open_file(
        &prepared,
        FileId(0),
        ScopePartials::File {
            evaluation: &[],
            publication: &[],
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
    )?;
    let read = |_| Err::<Rows<'_>, _>(OperatingError::NotRead);
    let outcome = file.rule(
        RuleIndex(0),
        RuleDemand {
            emit: EmitDemand::Enabled,
            observations: ObservationDemand::NotRequired,
        },
        &read,
        &mut symbols,
    )?;
    let result = match outcome.emit {
        RootOutcome::Complete(table) => Ok(table),
        RootOutcome::Failed(error) | RootOutcome::Unknown(error) => Err(error),
        _ => Err(OperatingError::NotRun),
    };
    file.finish()?;
    result
}
#[test]
fn a_key_finds_the_rows_that_have_it_in_input_order() {
    assert_eq!(
        indexed_join(1, 0).unwrap().cells,
        [Cell::Number(Integer(10)), Cell::Number(Integer(30))]
    );
}
#[test]
fn a_key_no_row_has_finds_nothing() {
    assert_eq!(indexed_join(9, 0).unwrap().count.0, 0);
}
#[test]
fn a_join_row_without_its_key_column_is_refused() {
    assert!(matches!(
        indexed_join(1, 3),
        Err(OperatingError::Table(RowsError::NoSuchColumn { index: 3 }))
    ));
}
#[test]
fn computed_inputs_use_the_common_typed_index_without_a_native_tree_finding() {
    assert_eq!(
        indexed_join(2, 0).unwrap().cells,
        [Cell::Number(Integer(20))]
    );
}
