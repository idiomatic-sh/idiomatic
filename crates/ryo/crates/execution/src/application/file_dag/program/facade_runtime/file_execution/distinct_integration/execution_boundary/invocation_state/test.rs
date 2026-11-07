//! Chained registered evaluations reuse downstream invocation state and release their peaks.
use super::super::super::scratch_owner::test::{execute, hosted};
use super::super::super::*;
use crate::domain::operating::types::Memo;
use ryo_values::domain::hosted::types::HostKind;
use ryo_values::domain::plan_parts::types::HostedNumber;
use ryo_values::domain::row_texts::types::Cell as HostedCell;
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
    let prepared = crate::application::file_dag::program::preparation::service::prepare(
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
        ObservationIdentity::Independent, crate::domain::executable_program::types::RoutePolicy::default(),
    )
    .unwrap();
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(std::sync::Arc::new(
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case())),
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
        ryo_values::domain::files::types::SourceView(b""),
        crate::application::file_dag::program::facade_runtime::types::ReuseEligibility::Eligible,
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
