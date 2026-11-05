//! Original execution negatives and state lifetime through the real owner.
use super::scratch_owner_test::{execute, hosted};
use super::*;
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
        Arc::new(CommonOperatorEngine::new(
            crate::outbound::operators::factory::test_port(),
        )),
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
        &[true],
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
        Arc::new(CommonOperatorEngine::new(
            crate::outbound::operators::factory::test_port(),
        )),
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
        &[true],
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
