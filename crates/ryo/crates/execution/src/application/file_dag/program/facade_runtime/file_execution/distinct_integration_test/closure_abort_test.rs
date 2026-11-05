//! A real downstream refusal interrupts partition construction and releases its backing.
use super::*;
use ryo_values::domain::{
    hosted::types::HostKind,
    plan_parts::types::{Comparison, HostedNumber, LiteralText, Operand, QueryLiteral, Test},
    records::types::Cell as HostCell,
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
        exercise(refusal);
    }
}
fn exercise(refusal: Refusal) {
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
                comparisons: vec![Comparison {
                    test: Test::Matches,
                    left: Operand::Literal(QueryLiteral::Text(LiteralText("a".into()))),
                    right: Operand::Column(ColumnIndex(1)),
                    origin: None,
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
    let patterns = if matches!(refusal, Refusal::Callback) {
        ["a", "a", "a"]
    } else {
        ["a", "[", "z"]
    };
    let host = super::scratch_owner_test::hosted(
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
        Arc::new(CommonOperatorEngine::new(Arc::new(
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
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
        &[true],
        services,
        b"",
        true,
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
            assert_eq!(
                rows.len(),
                1,
                "only the first partition may deliver before refusal"
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
    let (builds, last_backing) = owner.derived.scoped_observation();
    assert_eq!(
        builds,
        if matches!(refusal, Refusal::Callback) {
            1
        } else {
            2
        },
        "the third partition must not be built after the second refuses"
    );
    assert!(
        last_backing.upgrade().is_none(),
        "failed invocation releases its current partition backing"
    );
    file.finish().unwrap();
    assert_eq!(owner.derived.working_bytes(), 0);
    assert_eq!(owner.derived.publication_ownership_bytes().0, 0);
}
