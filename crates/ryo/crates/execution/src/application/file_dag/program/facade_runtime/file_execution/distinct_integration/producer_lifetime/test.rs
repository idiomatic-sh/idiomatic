//! Required derived storage follows real compiled producer consumers, including cancellation.
use super::super::*;
use ryo_relations::domain::relations::{
    service::to_bits,
    types::{ColumnsView, FactColumn},
};
use ryo_values::domain::rows::errors::TableModelError;

fn shared_plan(swapped: bool, invalid_terminal: bool, refuse_provider: bool) -> Plan {
    let mut steps = vec![
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Project {
                from: PlanInput::Relation(Relation::LanguageTarget),
                columns: vec![ColumnIndex(0), ColumnIndex(1), ColumnIndex(2)],
                distinct: RepeatedRows::Kept,
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
        // This actual reader first demands the first Join result, then the
        // shared producer, and finally a schema-valid unavailable population.
        // The cancellation therefore occurs after the first consumer released.
        if let Operation::Join { output, .. } = &mut steps[2].operation {
            *output = vec![
                JoinColumn {
                    side: Side::Left,
                    column: ColumnIndex(1),
                },
                JoinColumn {
                    side: Side::Right,
                    column: ColumnIndex(2),
                },
            ];
        }
        steps.push(PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Axis {
                from: PlanInput::Step(StepId(2)),
                edges: PlanInput::Step(StepId(1)),
                domain: PlanInput::Relation(Relation::SemanticChild),
                direction: ryo_values::domain::plan_parts::types::Direction::Descendant,
                id: ColumnIndex(0),
                parent: ColumnIndex(1),
                child: ColumnIndex(2),
                domain_id: ColumnIndex(2),
                domain_filter: vec![],
                domain_view: vec![],
                output: vec![],
            },
        });
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
    SkippedAtStart,
    InvalidTerminal,
    RefusingProvider,
}
fn exercise(final_consumer: FinalConsumer) -> Result<(), OperatingError> {
    let skipped_at_start = matches!(final_consumer, FinalConsumer::SkippedAtStart);
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
                diagnostic: if skipped_at_start {
                    DiagnosticDelivery::Forwarded
                } else {
                    DiagnosticDelivery::Collected
                },
                observations: ObservationDemand::NotRequired,
                capture: CaptureIntent::Disabled,
            },
            if skipped_at_start {
                FileRuleIntent::Skipped(crate::application::file_dag::program::admission::ApplicabilityReason::DiagnosticNotApplicable)
            } else {
                FileRuleIntent::Run {
                    diagnostic: DiagnosticDelivery::Collected,
                    observations: ObservationDemand::NotRequired,
                    capture: CaptureIntent::Disabled,
                }
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
    use crate::application::file_dag::program::runtime::ConsumerReleasePoint;
    let consumer = |rule: usize, step: usize| {
        let logical = prepared.program().rules()[rule].original_steps[step]
            .as_ref()
            .unwrap()
            .logical;
        prepared.program().logical_regions()[logical.0]
    };
    let first_consumer = consumer(0, 2);
    let final_consumer_region = consumer(1, if refuse_provider { 3 } else { 2 });
    file.execution
        .observe_producer_releases(producer, &[first_consumer, final_consumer_region])?;
    let first = if skipped_at_start {
        assert_eq!(file.execution.retry_checkpoint(producer)?.remaining.0, 1);
        let mut cells = vec![];
        file.emit_rows(RuleIndex(0), &read, &mut symbols, &mut |row| {
            cells.extend_from_slice(row);
            Ok(())
        })?;
        Table {
            kinds: vec![CellKind::Number; 2],
            count: RowCount(cells.len() / 2),
            cells,
        }
    } else {
        let RootOutcome::Complete(table) =
            file.rule(RuleIndex(0), demand(), &read, &mut symbols)?.emit
        else {
            return Err(OperatingError::NotRun);
        };
        table
    };
    assert_eq!(numbers(&first), [2, 9, 3, 7]);
    // Delivery is after canonical scheduling. The live interval is observed at
    // the actual consumer release, never inferred from API call order.
    let first_counts = file.owner.derived.counts();
    assert_eq!(first_counts.0, 1, "one actual equality construction");
    let reads_before_final = source_reads.get();
    if skipped_at_start {
        assert!(matches!(
            file.rule(RuleIndex(1), demand(), &read, &mut symbols),
            Err(OperatingError::NotRun)
        ));
    } else {
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
            FinalConsumer::SkippedAtStart => {
                unreachable!("upfront exclusion is handled before execution")
            }
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
                    final_counts.1 > 0,
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
    let events = file.execution.take_producer_releases();
    assert_eq!(events.len(), if skipped_at_start { 2 } else { 4 });
    let mut live_before = 0;
    let mut weak = None;
    for pair in events.chunks_exact(2) {
        let (before, after) = (&pair[0], &pair[1]);
        assert_eq!(before.point, ConsumerReleasePoint::Before);
        assert_eq!(after.point, ConsumerReleasePoint::After);
        assert_eq!(before.consumer, after.consumer);
        assert_eq!(
            before.checkpoint.remaining.0,
            after.checkpoint.remaining.0 + 1
        );
        if before.index_live {
            live_before += 1;
            assert_eq!(before.checkpoint.attempt, AttemptCheckpoint::Published);
            assert!(before.working_bytes > 0);
            let backing = before
                .index
                .as_ref()
                .expect("real acquired equality backing");
            if let Some(first) = weak {
                assert!(std::sync::Weak::ptr_eq(first, backing));
            } else {
                weak = Some(backing);
            }
            if after.checkpoint.remaining.0 > 0 {
                assert_eq!(after.checkpoint.attempt, AttemptCheckpoint::Published);
                assert!(
                    after.index_live,
                    "nonfinal actual consumption keeps backing live"
                );
                assert!(after.working_bytes > 0);
            }
        }
        if after.checkpoint.remaining.0 == 0 {
            assert_eq!(after.checkpoint.attempt, AttemptCheckpoint::Terminal);
            assert!(!after.index_live);
            assert_eq!(
                after.working_bytes, 0,
                "actual final release precedes output delivery and file finish"
            );
        }
    }
    // The invalid Project cannot fuse: canonical demand first completes its
    // actual Join reader, then refuses the Project's output column. Both readers
    // must therefore release at their real invocation boundaries.
    assert_eq!(live_before, if skipped_at_start { 1 } else { 2 });
    let weak = weak.expect("actual healthy consumer held the equality backing");
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
fn terminal_shape_failure_preserves_actual_reader_release_and_transferred_output() {
    exercise(FinalConsumer::InvalidTerminal).expect("actual compiled failure cancellation");
}

#[test]
fn final_refusing_consumer_releases_zero_budget_index_without_rereading_its_producer() {
    for final_consumer in [FinalConsumer::RefusingProvider] {
        exercise(final_consumer).expect("actual compiled final consumer cancellation");
    }
}

#[test]
fn upfront_skipped_reader_releases_the_real_shared_index_after_the_remaining_reader() {
    exercise(FinalConsumer::SkippedAtStart)
        .expect("actual zero-budget final reader release before file finish");
}
