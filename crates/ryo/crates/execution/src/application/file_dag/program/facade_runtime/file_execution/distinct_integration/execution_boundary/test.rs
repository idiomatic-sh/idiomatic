//! Original execution negatives and state lifetime through the real owner.
use super::super::scratch_owner::test::{execute, hosted};
use super::super::*;
use crate::domain::{
    operating::types::Memo,
    table_model::types::{SelectedRows, StoredSelection},
};
use ryo_relations::domain::relations::types::{ColumnsView, FactColumn};
use ryo_values::domain::{
    hosted::types::HostKind, plan_parts::types::HostedNumber, records::types::Cell as HostedCell,
};
fn source(rows: Rows<'_>) -> Result<Table, OperatingError> {
    let mut plan = plan(Scenario::ProjectRetry);
    plan.steps.truncate(1);
    plan.emit.step = StepId(0);
    plan.steps[0].operation = Operation::Select {
        from: PlanInput::Relation(Relation::SyntaxTree),
        comparisons: vec![],
        kept: vec![],
    };
    let derived = crate::application::file_dag::testing::program_derived_port();
    let prepared = facade::prepare(
        Compiled {
            schedule: RuleSchedule::Independent,
            observations: vec![vec![]],
            plans: vec![plan],
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
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
        ))),
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("execution-boundary.py".into()),
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
        b"",
        true,
        &mut symbols,
        None,
        None,
    )?;
    let read = |relation| {
        if relation == Relation::SyntaxTree {
            Ok(rows.clone())
        } else {
            Err(OperatingError::NotRead)
        }
    };
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
fn malformed_declared_population_is_refused_not_silently_shortened() {
    let table = Table {
        kinds: kinds(Relation::SyntaxTree).to_vec(),
        cells: vec![Cell::Node(ryo_relations::domain::relations::types::NodeId(
            0,
        ))],
        count: RowCount(2),
    };
    assert!(matches!(
        source(Rows::Computed(&table)),
        Err(OperatingError::ShapeMismatch)
    ));
}
#[test]
fn zero_width_bags_keep_their_declared_population() {
    let (result, _) = execute(
        Operation::Project {
            from: PlanInput::Step(StepId(0)),
            columns: vec![],
            distinct: RepeatedRows::Kept,
        },
        hosted(&[], vec![vec![], vec![], vec![]]),
        &mut Symbols::default(),
    );
    let output = result.unwrap();
    assert_eq!(output.count.0, 3);
    assert!(output.cells.is_empty());
}
struct MemoSink {
    starts: Vec<usize>,
    rows: Vec<Cell>,
}
impl RowSink for MemoSink {
    fn row(
        &mut self,
        context: &mut OperatorContext<'_>,
        row: &[Cell],
    ) -> Result<(), OperatingError> {
        self.starts.push(context.memo.len());
        context.memo.push(Memo::Holds);
        self.rows.extend_from_slice(row);
        Ok(())
    }
}
#[test]
fn chained_registered_evaluations_reuse_downstream_invocation_state() {
    let mut plan = plan(Scenario::ProjectRetry);
    plan.steps = vec![
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Hosted {
                relation: HostedNumber(0),
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: PlanInput::Step(StepId(0)),
                comparisons: vec![],
                kept: vec![],
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Project {
                from: PlanInput::Step(StepId(1)),
                columns: vec![ColumnIndex(0)],
                distinct: RepeatedRows::Kept,
            },
        },
    ];
    plan.emit.step = StepId(2);
    let host = hosted(
        &[HostKind::Number],
        (1..=3)
            .map(|number| vec![HostedCell::Number(Integer(number))])
            .collect(),
    );
    let derived = crate::application::file_dag::testing::program_derived_port();
    let prepared = facade::prepare(
        Compiled {
            schedule: RuleSchedule::Independent,
            observations: vec![vec![]],
            plans: vec![plan],
            bindings: vec![vec![]],
        },
        &[rule()],
        &[Digest([0; 16])],
        &[host],
        Arc::clone(&derived),
        ObservationIdentity::Independent,
    )
    .unwrap();
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(std::sync::Arc::new(
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
        ))),
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("chained-state.py".into()),
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
        b"",
        true,
        &mut symbols,
        None,
        None,
    )
    .unwrap();
    let read = |_| Err::<Rows<'_>, _>(OperatingError::NotRead);
    let mut sink = MemoSink {
        starts: vec![],
        rows: vec![],
    };
    forward(&mut file, &read, &mut symbols, &mut sink).unwrap();
    assert_eq!(sink.starts, [0, 1, 2]);
    assert_eq!(
        sink.rows,
        (1..=3)
            .map(|number| Cell::Number(Integer(number)))
            .collect::<Vec<_>>()
    );
    file.finish().unwrap();
}
#[test]
fn partition_observations_keep_large_then_small_peak_and_release() {
    let mut rows = (0..32)
        .map(|at| {
            vec![
                HostedCell::Number(Integer(at)),
                HostedCell::Number(Integer(at + 1)),
                HostedCell::Number(Integer(0)),
            ]
        })
        .collect::<Vec<_>>();
    rows.push(vec![
        HostedCell::Number(Integer(100)),
        HostedCell::Number(Integer(101)),
        HostedCell::Number(Integer(1)),
    ]);
    let (result, observed) = execute(
        Operation::Closure {
            edges: PlanInput::Step(StepId(0)),
            keys: vec![ColumnIndex(2)],
            source: ColumnIndex(0),
            destination: ColumnIndex(1),
        },
        hosted(&[HostKind::Number; 3], rows),
        &mut Symbols::default(),
    );
    assert_eq!(result.unwrap().count.0, 32 * 33 / 2 + 1);
    assert_eq!(observed.closure_partition_rows, 32);
    assert!(observed.closure_partition_capacity >= 64);
    // execute calls normal file.finish and checks the real owner's required bytes are zero.
}
#[test]
fn ragged_selected_fallback_cannot_admit_mismatched_projection_kinds() {
    let columns = [
        FactColumn(vec![0, 1]),
        FactColumn(vec![0]),
        FactColumn(vec![0, 0]),
        FactColumn(vec![0, 0]),
        FactColumn(vec![0, 0]),
    ];
    let rows = Rows::Selected(SelectedRows {
        source: ColumnsView {
            kinds: kinds(Relation::SyntaxTree),
            columns: &columns,
        },
        selection: Arc::new(StoredSelection {
            relation: Relation::SyntaxTree,
            rows: Arc::new(vec![0]),
            columns: vec![ColumnIndex(0)],
            kinds: vec![CellKind::Symbol],
            identity: Arc::new(Default::default()),
        }),
    });
    assert!(matches!(source(rows), Err(OperatingError::ShapeMismatch)));
}

fn output_requests(
    counts: &[usize],
    bad_first: bool,
) -> (Arc<facade::PreparedProgram>, Arc<ProgramServices>) {
    use ryo_values::domain::hosted::types::{
        HostColumn, HostColumnName, HostName, HostRelation, HostRow,
    };
    let hosts = counts
        .iter()
        .enumerate()
        .map(|(index, count)| HostRelation {
            name: HostName(format!("output-{index}").into()),
            columns: vec![HostColumn {
                name: HostColumnName("value".into()),
                kind: HostKind::Number,
            }],
            rows: (0..*count)
                .map(|_| HostRow(vec![HostedCell::Number(Integer(7))]))
                .collect(),
        })
        .collect::<Vec<_>>();
    let plans = counts
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let mut plan = plan(Scenario::ProjectRetry);
            plan.steps[0].operation = Operation::Hosted {
                relation: HostedNumber(index as u32),
            };
            plan.steps[1].operation = Operation::Project {
                from: PlanInput::Step(StepId(0)),
                columns: vec![ColumnIndex(if bad_first && index == 0 { 99 } else { 0 })],
                distinct: RepeatedRows::Kept,
            };
            plan
        })
        .collect();
    let rules = counts
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let mut rule = rule();
            rule.id = RuleId(format!("output-{index}").into());
            rule
        })
        .collect::<Vec<_>>();
    let derived = crate::application::file_dag::testing::program_derived_port();
    let prepared = facade::prepare(
        Compiled {
            schedule: RuleSchedule::Independent,
            observations: vec![vec![]; counts.len()],
            plans,
            bindings: vec![vec![]; counts.len()],
        },
        &rules,
        &vec![Digest([0; 16]); counts.len()],
        &hosts,
        Arc::clone(&derived),
        ObservationIdentity::Independent,
    )
    .unwrap();
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(Arc::new(
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
        ))),
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("owned-outputs.py".into()),
            size: FileSize(0),
            modified: Nanoseconds(0),
        })],
    ));
    (prepared, services)
}
#[test]
fn large_tiny_and_empty_rule_outputs_are_owned_beyond_workspace_drop() {
    let counts = [4096, 1, 0, 3];
    let (prepared, services) = output_requests(&counts, false);
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
        }; 4],
        services,
        b"",
        true,
        &mut symbols,
        None,
        None,
    )
    .unwrap();
    let read = |_| Err::<Rows<'_>, _>(OperatingError::NotRead);
    let mut outputs = Vec::new();
    for (index, &count) in counts.iter().enumerate() {
        let outcome = file
            .rule(
                RuleIndex(index),
                RuleDemand {
                    emit: EmitDemand::Enabled,
                    observations: ObservationDemand::NotRequired,
                },
                &read,
                &mut symbols,
            )
            .unwrap();
        let RootOutcome::Complete(table) = outcome.emit else {
            panic!("owned output required")
        };
        assert_eq!(
            table.kinds,
            [CellKind::Number],
            "empty outputs retain the actual prepared schema"
        );
        assert_eq!(table.count.0, count);
        assert_eq!(table.cells, vec![Cell::Number(Integer(7)); count]);
        outputs.push(table);
    }
    let pointers = outputs
        .iter()
        .map(|table| table.cells.as_ptr())
        .collect::<Vec<_>>();
    file.finish().unwrap();
    for ((table, &count), pointer) in outputs.iter().zip(&counts).zip(pointers) {
        assert_eq!(table.count.0, count);
        assert_eq!(table.kinds, [CellKind::Number]);
        assert_eq!(table.cells, vec![Cell::Number(Integer(7)); count]);
        assert_eq!(
            table.cells.as_ptr(),
            pointer,
            "the returned table keeps its own allocation after actual owner finish"
        );
    }
}
#[test]
fn evaluation_errors_do_not_leave_mutable_state_in_the_next_rule() {
    let (prepared, services) = output_requests(&[2, 2], true);
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
        }; 2],
        services,
        b"",
        true,
        &mut symbols,
        None,
        None,
    )
    .unwrap();
    let read = |_| Err::<Rows<'_>, _>(OperatingError::NotRead);
    let demand = || RuleDemand {
        emit: EmitDemand::Enabled,
        observations: ObservationDemand::NotRequired,
    };
    let failed = file
        .rule(RuleIndex(0), demand(), &read, &mut symbols)
        .unwrap();
    assert!(matches!(
        failed.emit,
        RootOutcome::Failed(OperatingError::Table(
            ryo_values::domain::rows::errors::TableModelError::NoSuchColumn { index: 99 }
        ))
    ));
    let next = file
        .rule(RuleIndex(1), demand(), &read, &mut symbols)
        .unwrap();
    let RootOutcome::Complete(table) = next.emit else {
        panic!("independent next rule must complete")
    };
    assert_eq!(table.count.0, 2);
    assert_eq!(table.kinds, [CellKind::Number]);
    assert_eq!(table.cells, [Cell::Number(Integer(7)); 2]);
    let repeated = file
        .rule(RuleIndex(0), demand(), &read, &mut symbols)
        .unwrap();
    assert!(
        matches!(
            repeated.emit,
            RootOutcome::Failed(OperatingError::Table(
                ryo_values::domain::rows::errors::TableModelError::NoSuchColumn { index: 99 }
            ))
        ),
        "the earlier failure remains exact after a later successful invocation"
    );
    file.finish().unwrap();
    assert_eq!(table.cells, [Cell::Number(Integer(7)); 2]);
}
