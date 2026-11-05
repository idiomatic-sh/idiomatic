//! Required derived storage follows real compiled producer consumers, including cancellation.
use super::*;
use ryo_relations::domain::relations::{
    service::to_bits,
    types::{ColumnsView, FactColumn},
};
use ryo_values::domain::rows::errors::TableModelError;

fn shared_plan(swapped: bool, invalid_terminal: bool, refuse_provider: bool) -> Plan {
    let mut steps = vec![
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Extend {
                from: PlanInput::Relation(Relation::LanguageTarget),
                values: vec![ryo_values::domain::plan_parts::types::Operand::Literal(
                    ryo_values::domain::plan_parts::types::QueryLiteral::Number(Integer(42)),
                )],
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Union {
                left: PlanInput::Step(StepId(0)),
                right: PlanInput::Step(StepId(0)),
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Join {
                left: PlanInput::Step(StepId(0)),
                right: PlanInput::Step(StepId(1)),
                left_on: vec![ColumnIndex(1)],
                right_on: vec![ColumnIndex(1)],
                output: vec![
                    JoinColumn {
                        side: Side::Left,
                        column: ColumnIndex(1 + u32::from(swapped)),
                    },
                    JoinColumn {
                        side: Side::Right,
                        column: ColumnIndex(1 + u32::from(!swapped)),
                    },
                ],
                filter: vec![],
                view: vec![],
                reach: None,
            },
        },
    ];
    if refuse_provider {
        if let Operation::Join {
            left,
            left_on,
            right_on,
            ..
        } = &mut steps[2].operation
        {
            *left = PlanInput::Relation(Relation::CompilerFeature);
            *left_on = vec![ColumnIndex(0)];
            *right_on = vec![ColumnIndex(0)];
        }
    }
    if invalid_terminal {
        steps.push(PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Project {
                from: PlanInput::Step(StepId(2)),
                columns: vec![ColumnIndex(99)],
                distinct: RepeatedRows::Kept,
            },
        });
    }
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
fn numbers(table: &Table) -> Vec<i64> {
    table
        .cells
        .iter()
        .map(|value| match value {
            Cell::Number(Integer(number)) => *number,
            other => panic!("number output required, got {other:?}"),
        })
        .collect()
}
#[derive(Clone, Copy)]
enum FinalConsumer {
    Completes,
    InvalidTerminal,
    RefusingProvider,
}
fn exercise(final_consumer: FinalConsumer) -> Result<(), OperatingError> {
    let invalid_terminal = matches!(final_consumer, FinalConsumer::InvalidTerminal);
    let refuse_provider = matches!(final_consumer, FinalConsumer::RefusingProvider);
    let derived = crate::application::file_dag::testing::program_derived_port();
    let mut second = rule();
    second.id = RuleId("second-derived-consumer".into());
    let prepared = facade::prepare(
        Compiled {
            schedule: RuleSchedule::Independent,
            observations: vec![vec![], vec![]],
            plans: vec![
                shared_plan(false, false, false),
                shared_plan(true, invalid_terminal, refuse_provider),
            ],
            bindings: vec![vec![], vec![]],
        },
        &[rule(), second],
        &[Digest([0; 16]), Digest([1; 16])],
        &[],
        Arc::clone(&derived),
        ObservationIdentity::Independent,
    )?;
    let shared = |rule: usize| {
        let logical = prepared.program().rules()[rule].original_steps[1]
            .as_ref()
            .unwrap()
            .logical;
        prepared.program().logical_regions()[logical.0]
    };
    let producer = shared(0);
    assert_eq!(
        producer,
        shared(1),
        "both joins must demand the same actual computed producer"
    );
    assert_eq!(prepared.program().regions()[producer.0].consumers, 2);
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(std::sync::Arc::new(
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
        ))),
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("shared-producer.py".into()),
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
        &[
            FileRuleIntent::Run {
                roots: RuleDemand {
                    emit: EmitDemand::Enabled,
                    observations: ObservationDemand::NotRequired,
                },
                capture: CaptureIntent::Disabled,
            },
            FileRuleIntent::Run {
                roots: RuleDemand {
                    emit: EmitDemand::Enabled,
                    observations: ObservationDemand::NotRequired,
                },
                capture: CaptureIntent::Disabled,
            },
        ],
        services,
        b"",
        true,
        &mut symbols,
        None,
        None,
    )?;
    // Required views still have to live until their published producer's last
    // consumer when optional retention is unavailable.
    file.owner.derived.set_optional_limit(0)?;
    let columns = [
        FactColumn(vec![to_bits(Cell::File(FileId(0))).0; 2]),
        FactColumn(vec![
            to_bits(Cell::Number(Integer(2))).0,
            to_bits(Cell::Number(Integer(3))).0,
        ]),
        FactColumn(vec![
            to_bits(Cell::Number(Integer(9))).0,
            to_bits(Cell::Number(Integer(7))).0,
        ]),
    ];
    let source_reads = std::cell::Cell::new(0);
    let read = |relation| {
        if relation == Relation::LanguageTarget {
            source_reads.set(source_reads.get() + 1);
            Ok(Rows::Stored(ColumnsView {
                kinds: kinds(relation),
                columns: &columns,
            }))
        } else {
            Err(OperatingError::NotRead)
        }
    };
    let demand = || RuleDemand {
        emit: EmitDemand::Enabled,
        observations: ObservationDemand::NotRequired,
    };
    let first = file.rule(RuleIndex(0), demand(), &read, &mut symbols)?;
    let RootOutcome::Complete(first) = first.emit else {
        return Err(OperatingError::NotRun);
    };
    assert_eq!(numbers(&first), [2, 9, 3, 7]);
    assert!(
        file.owner.derived.working_bytes() > 0,
        "a nonfinal consumer must leave the shared producer's required index live"
    );
    let after_first = file.execution.retry_checkpoint(producer)?;
    assert_eq!(after_first.attempt, AttemptCheckpoint::Published);
    assert_eq!(after_first.remaining.0, 1);
    let first_counts = file.owner.derived.counts();
    assert_eq!(first_counts.0, 1, "one actual equality construction");
    let weak = file
        .owner
        .derived
        .view_weak(ryo_derived::domain::contract::types::DerivedKind::Equality)
        .expect("actual index backing from the first compiled consumer");
    assert!(weak.upgrade().is_some());
    assert_eq!(file.owner.derived.retained_bytes(), 0);
    let reads_before_final = source_reads.get();
    {
        let second = file.rule(RuleIndex(1), demand(), &read, &mut symbols)?;
        match final_consumer {
            FinalConsumer::InvalidTerminal => {
                assert!(
                    matches!(
                        second.emit,
                        RootOutcome::Failed(OperatingError::Table(TableModelError::NoSuchColumn {
                            index: 99
                        }))
                    ),
                    "terminal schema failure preserves its exact error while cancelling the unread shared dependency"
                );
            }
            FinalConsumer::RefusingProvider => assert!(
                matches!(second.emit, RootOutcome::Failed(OperatingError::NotRead)),
                "the real provider's original refusal must survive consumer cleanup"
            ),
            FinalConsumer::Completes => {
                let RootOutcome::Complete(second) = second.emit else {
                    return Err(OperatingError::NotRun);
                };
                assert_eq!(numbers(&second), [9, 2, 7, 3]);
                let final_counts = file.owner.derived.counts();
                assert_eq!(
                    final_counts.0, first_counts.0,
                    "shared producer index is built once"
                );
                assert!(
                    final_counts.1 > first_counts.1,
                    "second consumer reuses the same live index"
                );
            }
        }
    }
    assert_eq!(
        source_reads.get(),
        reads_before_final,
        "the shared source is not reread for a cache hit, skipped consumer or refusing final consumer"
    );
    let final_state = file.execution.retry_checkpoint(producer)?;
    assert_eq!(final_state.attempt, AttemptCheckpoint::Terminal);
    assert_eq!(final_state.remaining.0, 0);
    assert_eq!(
        file.owner.derived.working_bytes(),
        0,
        "final consumption or cancellation releases required storage before file shutdown"
    );
    assert_eq!(
        numbers(&first),
        [2, 9, 3, 7],
        "transferred output survives producer retirement"
    );
    assert!(
        weak.upgrade().is_none(),
        "last actual consumer releases the unretained physical index"
    );
    assert_eq!(file.owner.derived.owned_bytes(), 0);
    file.finish()?;
    Ok(())
}
#[test]
fn shared_published_producer_keeps_index_until_its_final_real_consumer() {
    exercise(FinalConsumer::Completes).expect("actual compiled producer lifetime");
}
#[test]
fn failed_terminal_cancels_the_shared_producer_and_releases_its_index() {
    exercise(FinalConsumer::InvalidTerminal).expect("actual compiled failure cancellation");
}

#[test]
fn final_refusing_consumer_releases_zero_budget_global_index_without_reading_sources() {
    for final_consumer in [FinalConsumer::RefusingProvider] {
        exercise(final_consumer).expect("actual compiled final consumer cancellation");
    }
}
