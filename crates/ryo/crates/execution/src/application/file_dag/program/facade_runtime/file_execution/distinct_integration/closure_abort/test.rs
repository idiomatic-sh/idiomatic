//! A real downstream refusal interrupts partition construction and releases its backing.
use super::super::*;
use ryo_values::domain::{
    hosted::types::HostKind,
    plan_parts::types::{HostedNumber, LiteralText, Operand, PlanComparison, QueryLiteral, Test},
    row_texts::types::Cell as HostCell,
};

#[derive(Clone, Copy)]
enum Refusal {
    CollectedPattern,
    DeliveredPattern,
    Callback,
}
#[test]
fn later_partition_refusal_stops_work_and_releases_backing() {
    for refusal in [
        Refusal::CollectedPattern,
        Refusal::DeliveredPattern,
        Refusal::Callback,
    ] {
        for fused in [false, true] {
            exercise(refusal, fused);
        }
    }
}
fn exercise(refusal: Refusal, fused: bool) {
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
            operation: Operation::Closure {
                edges: PlanInput::Step(StepId(0)),
                keys: vec![ColumnIndex(2), ColumnIndex(3)],
                source: ColumnIndex(0),
                destination: ColumnIndex(1),
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: PlanInput::Step(StepId(1)),
                comparisons: vec![PlanComparison {
                    test: Test::Matches,
                    left: Operand::Literal(QueryLiteral::Text(LiteralText("a".into()))),
                    right: Operand::Column(ColumnIndex(1)),
                    origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                }],
                kept: vec![
                    ColumnIndex(0),
                    ColumnIndex(1),
                    ColumnIndex(2),
                    ColumnIndex(3),
                ],
            },
        },
    ];
    plan.emit.step = StepId(2);
    if fused {
        let Operation::Select { kept, .. } = &mut plan.steps[2].operation else {
            unreachable!()
        };
        kept.clear(); // All four columns still pass through; this is the actual fusible shape.
    }
    let patterns = if matches!(refusal, Refusal::Callback) {
        ["a", "a", "a"]
    } else {
        ["a", "[", "z"]
    };
    let host = super::super::scratch_owner::test::hosted(
        &[
            HostKind::Number,
            HostKind::Number,
            HostKind::Number,
            HostKind::Text,
        ],
        patterns
            .into_iter()
            .enumerate()
            .map(|(key, pattern)| {
                vec![
                    HostCell::Number(Integer(1)),
                    HostCell::Number(Integer(2)),
                    HostCell::Number(Integer(key as i64)),
                    HostCell::Text(pattern.into()),
                ]
            })
            .collect(),
    );
    let recorded = Arc::new(
        ryo_execution_store::application::file_dag::derived::owner::fixtures::resources::RecordedDerived::default(
        ),
    );
    let derived: Arc<dyn ryo_execution_store::outbound::derived::interface::DerivedPort> = recorded.clone();
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
        ObservationIdentity::Independent,
    )
    .unwrap();
    let program = prepared.program();
    let physical = |step: usize| {
        let logical = program.rules()[0].original_steps[step]
            .as_ref()
            .unwrap()
            .logical;
        program.logical_regions()[logical.0]
    };
    assert_eq!(
        physical(1) == physical(2),
        fused,
        "the test must observe the actual fused or materialized compiled boundary"
    );
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(Arc::new(
            crate::application::file_dag::fixtures::operators(),
        ))),
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("closure-abort.py".into()),
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
            diagnostic: if matches!(refusal, Refusal::CollectedPattern) {
                DiagnosticDelivery::Collected
            } else {
                DiagnosticDelivery::Forwarded
            },
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        }],
        services,
        ryo_values::domain::files::types::SourceView(b""),
        ryo_execution_store::application::file_dag::relations::types::ReuseEligibility::Eligible,
        &mut symbols,
        None,
        None,
    )
    .unwrap();
    let owner = Rc::clone(&file.owner);
    let read = |_| Err::<Rows<'_>, _>(OperatingError::NotRead);
    match refusal {
        Refusal::CollectedPattern => {
            let result = file
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
            assert!(matches!(
                result.emit,
                RootOutcome::Failed(OperatingError::InvalidRegex { .. })
            ));
        }
        Refusal::DeliveredPattern | Refusal::Callback => {
            let mut rows = Vec::new();
            let result = file.emit_rows(RuleIndex(0), &read, &mut symbols, &mut |row| {
                rows.push(row.to_vec());
                if matches!(refusal, Refusal::Callback) {
                    Err(OperatingError::WrongBinding)
                } else {
                    Ok(())
                }
            });
            match refusal {
                Refusal::Callback => assert!(matches!(result, Err(OperatingError::WrongBinding))),
                _ => assert!(matches!(result, Err(OperatingError::InvalidRegex { .. }))),
            }
            if matches!(refusal, Refusal::DeliveredPattern) {
                assert!(
                    rows.is_empty(),
                    "a later semantic failure is atomic for callbacks too"
                );
            } else {
                assert_eq!(
                    rows.len(),
                    1,
                    "completed publication delivery stops at the first sink refusal"
                );
                assert_eq!(rows[0].len(), 4);
                assert_eq!(rows[0][0], Cell::Number(Integer(0)));
                assert_eq!(
                    rows[0][2..],
                    [Cell::Number(Integer(1)), Cell::Number(Integer(2))]
                );
                let Cell::Symbol(symbol) = rows[0][1] else {
                    panic!("hosted text retains its symbol")
                };
                assert_eq!(
                    ryo_language::domain::symbols::service::text_of(
                        &symbols,
                        ryo_language::domain::symbols::types::ByteSlice(b""),
                        symbol
                    )
                    .unwrap()
                    .0,
                    "a"
                );
            }
        }
    }
    let (builds, last_backing) = owner.derived().scoped_observation();
    assert_eq!(
        builds,
        if fused && !matches!(refusal, Refusal::Callback) {
            2
        } else {
            3
        },
        "only a real fused continuation can stop partition construction at its semantic failure"
    );
    let calls = recorded.builds();
    assert!(
        matches!(calls.first(), Some(ryo_derived::domain::request::types::DerivedRequest::PartitionOrder { keys })
        if keys == &[ColumnIndex(2), ColumnIndex(3)])
    );
    assert_eq!(
        calls.len(),
        1 + builds,
        "the shared partition order is a separate family from actual scoped adjacency construction"
    );
    assert!(calls[1..].iter().all(|request| matches!(
        request,
        ryo_derived::domain::request::types::DerivedRequest::ClosureAdjacency {
            source: ColumnIndex(0),
            destination: ColumnIndex(1)
        }
    )));
    assert!(
        last_backing.upgrade().is_none(),
        "failed invocation releases its current partition backing"
    );
    file.finish().unwrap();
    assert_eq!(owner.derived().working_bytes(), 0);
    assert_eq!(owner.derived().publication_ownership_bytes().0, 0);
}
