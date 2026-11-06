//! Rule outputs outlive workspace drops, and evaluation errors leave no mutable state behind.
use super::super::super::*;
use ryo_values::domain::hosted::types::HostKind;
use ryo_values::domain::plan_parts::types::HostedNumber;
use ryo_values::domain::records::types::Cell as HostedCell;
fn output_requests(
    outputs: &[(&str, usize)],
    bad_first: bool,
) -> (
    Arc<crate::application::file_dag::program::preparation::types::PreparedProgram>,
    Arc<ProgramServices>,
) {
    use ryo_values::domain::hosted::types::{
        HostColumn, HostColumnName, HostName, HostRelation, HostRow,
    };
    let hosts = outputs
        .iter()
        .map(|(name, count)| HostRelation {
            name: HostName(format!("output-{name}").into()),
            columns: vec![HostColumn {
                name: HostColumnName("value".into()),
                kind: HostKind::Number,
            }],
            rows: (0..*count)
                .map(|_| HostRow(vec![HostedCell::Number(Integer(7))]))
                .collect(),
        })
        .collect::<Vec<_>>();
    let plans = outputs
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
    let rules = outputs
        .iter()
        .map(|(name, _)| {
            let mut rule = rule();
            rule.id = RuleId(format!("output-{name}").into());
            rule
        })
        .collect::<Vec<_>>();
    let derived = crate::application::file_dag::testing::program_derived_port();
    let prepared = crate::application::file_dag::program::preparation::service::prepare(
        Compiled {
            schedule: RuleSchedule::Independent,
            observations: vec![vec![]; outputs.len()],
            plans,
            bindings: vec![vec![]; outputs.len()],
        },
        &rules,
        &vec![Digest([0; 16]); outputs.len()],
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
    let outputs = [("large", 4096), ("tiny", 1), ("empty", 0), ("small", 3)];
    let counts = outputs.map(|(_, count)| count);
    let (prepared, services) = output_requests(&outputs, false);
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
        ryo_values::domain::files::types::SourceView(b""),
        crate::application::file_dag::program::facade_runtime::types::ReuseEligibility::Eligible,
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
    let (prepared, services) = output_requests(&[("failing", 2), ("following", 2)], true);
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
        ryo_values::domain::files::types::SourceView(b""),
        crate::application::file_dag::program::facade_runtime::types::ReuseEligibility::Eligible,
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
