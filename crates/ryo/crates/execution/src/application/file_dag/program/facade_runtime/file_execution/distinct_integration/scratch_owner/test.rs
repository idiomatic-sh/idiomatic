//! Actual invocation storage measurements through normal compiled file execution.
use super::super::*;
use crate::domain::operating::types::ExecutionObservation;
use ryo_values::domain::{
    hosted::types::{HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow},
    plan_parts::types::{HostedNumber, Operand, PlanComparison, Test},
    row_texts::types::Cell as HostedCell,
};
pub(in super::super) fn hosted(kinds: &[HostKind], rows: Vec<Vec<HostedCell>>) -> HostRelation {
    HostRelation {
        name: HostName("scratch-input".into()),
        columns: kinds
            .iter()
            .enumerate()
            .map(|(at, kind)| HostColumn {
                name: HostColumnName(format!("value-{at}").into()),
                kind: *kind,
            })
            .collect(),
        rows: rows.into_iter().map(HostRow).collect(),
    }
}
pub(in super::super) fn execute(
    operation: Operation,
    host: HostRelation,
    symbols: &mut Symbols,
) -> (Result<Table, OperatingError>, ExecutionObservation) {
    let mut plan = plan(Scenario::ProjectRetry);
    plan.steps[0].operation = Operation::Hosted {
        relation: HostedNumber(0),
    };
    plan.steps[1].operation = operation;
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
            crate::outbound::operators::fixtures::fake::OperatorsFake::default(),
        ))),
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("scratch-owner.py".into()),
            size: FileSize(0),
            modified: Nanoseconds(0),
        })],
    ));
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
        crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
        symbols,
        None,
        None,
    )
    .unwrap();
    let read = |_| Err::<Rows<'_>, _>(OperatingError::NotRead);
    let outcome = file
        .rule(
            RuleIndex(0),
            RuleDemand {
                emit: EmitDemand::Enabled,
                observations: ObservationDemand::NotRequired,
            },
            &read,
            symbols,
        )
        .unwrap();
    let observed = file.route_observation(RuleIndex(0), StepId(1)).unwrap();
    let context = file
        .context
        .borrow(symbols, &prepared, RuleIndex(0))
        .unwrap();
    assert!(
        context.scratch.join_scratch.0.get().is_none(),
        "no stale join cluster escapes the actual invocation workspace"
    );
    assert!(
        context.scratch.select_scratch.0.get().is_none(),
        "dynamic matcher state belongs to the actual invocation, not reusable rule scratch"
    );
    let owner = Rc::clone(&file.owner);
    file.finish().unwrap();
    assert_eq!(
        owner.derived().working_bytes(),
        0,
        "normal file shutdown releases actual required resources"
    );
    let result = match outcome.emit {
        RootOutcome::Complete(table) => Ok(table),
        RootOutcome::Unknown(error) | RootOutcome::Failed(error) => Err(error),
        _ => Err(OperatingError::NotRun),
    };
    (result, observed)
}
#[test]
fn projection_constructs_only_the_group_it_uses_and_keeps_bags() {
    for (distinct, expected, groups) in [
        (RepeatedRows::Kept, vec![2, 1, 2], 0),
        (RepeatedRows::Dropped, vec![2, 1], 1),
    ] {
        let host = hosted(
            &[HostKind::Number],
            [2, 1, 2]
                .map(|number| vec![HostedCell::Number(Integer(number))])
                .into(),
        );
        let (result, observed) = execute(
            Operation::Project {
                from: PlanInput::Step(StepId(0)),
                columns: vec![ColumnIndex(0)],
                distinct,
            },
            host,
            &mut Symbols::default(),
        );
        assert_eq!(
            result.unwrap().cells,
            expected
                .into_iter()
                .map(|number| Cell::Number(Integer(number)))
                .collect::<Vec<_>>()
        );
        assert_eq!(observed.owners_initialized, groups);
    }
}
#[test]
fn dynamic_regex_initialization_is_execution_local_and_lazy() {
    for populated in [false, true, false] {
        let rows = if populated {
            vec![vec![HostedCell::Text("[".into())]]
        } else {
            vec![]
        };
        let (result, observed) = execute(
            Operation::Select {
                from: PlanInput::Step(StepId(0)),
                comparisons: vec![PlanComparison {
                    test: Test::Matches,
                    left: Operand::Column(ColumnIndex(0)),
                    right: Operand::Column(ColumnIndex(0)),
                    origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                }],
                kept: vec![ColumnIndex(0)],
            },
            hosted(&[HostKind::Text], rows),
            &mut Symbols::default(),
        );
        if populated {
            assert!(matches!(result, Err(OperatingError::InvalidRegex { .. })));
        } else {
            assert_eq!(result.unwrap().count.0, 0);
        }
        assert_eq!(
            observed.regexes, 0,
            "invalid dynamic patterns are never cached"
        );
    }
}
#[test]
fn aggregation_resources_are_engine_owned_and_released() {
    use crate::application::file_dag::acceptance::operator_contract_test::fixtures::{
        expectations::assert_expected, fixture,
    };
    use ryo_values::domain::query_operators::types::OperationKind;
    for kind in [OperationKind::GroupCount, OperationKind::GroupDigest] {
        let mut case = fixture(kind);
        let input = &case.inputs[0];
        let rows = input
            .cells
            .chunks_exact(input.kinds.len())
            .map(|row| {
                row.iter()
                    .map(|value| match value {
                        Cell::Number(number) => HostedCell::Number(*number),
                        other => panic!("numeric aggregation fixture, got {other:?}"),
                    })
                    .collect()
            })
            .collect();
        let operation = match case.operation {
            Operation::GroupCount { keys, distinct, .. } => Operation::GroupCount {
                from: PlanInput::Step(StepId(0)),
                keys,
                distinct,
            },
            Operation::GroupDigest { keys, over, .. } => Operation::GroupDigest {
                from: PlanInput::Step(StepId(0)),
                keys,
                over,
            },
            _ => unreachable!(),
        };
        let (result, observed) = execute(
            operation,
            hosted(&vec![HostKind::Number; input.kinds.len()], rows),
            &mut case.symbols,
        );
        let out = result.unwrap();
        let rows = out
            .cells
            .chunks_exact(out.kinds.len())
            .map(<[_]>::to_vec)
            .collect::<Vec<_>>();
        assert_expected(&case.expected, &rows, &case.symbols);
        assert_eq!(observed.owners_initialized, 1, "{kind:?}");
    }
}
