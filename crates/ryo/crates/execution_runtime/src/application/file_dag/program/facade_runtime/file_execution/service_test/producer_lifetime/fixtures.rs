//! Actual compiled producer plans shared by the internal and external reader controls.
use super::super::super::*;
use super::super::fixtures::*;
use crate::application::file_dag::execution::CommonOperatorEngine;
use crate::application::file_dag::program::runtime::types::AttemptCheckpoint;
use crate::application::file_dag::program::runtime::types::IndexLiveness;
use ryo_execution_domain::domain::{
    observation_schedule::types::RuleSchedule,
    prepared_request::types::{Compiled, ObservationIdentity},
};
use ryo_execution_store::application::file_dag::derived::owner::witness::service::{
    BackingIdentity, BackingLiveness, BackingWitness,
};
use ryo_execution_store::application::file_dag::service::ScopePartials;
use ryo_language::domain::{files::types::FilePath, symbols::types::Symbols};
use ryo_relations::domain::relations::types::{Integer, Relation, Value as Cell};
use ryo_relations::domain::relations::{
    service::kinds,
    types::{CellKind, FileId},
};
use ryo_relations::domain::relations::{
    service::to_bits,
    types::{ColumnsView, FactColumn},
};
use ryo_values::domain::rows::errors::RowsError;
use ryo_values::domain::{
    files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
    rows::types::RowCount,
    runs::types::RuleId,
    stored_facts::types::Digest,
};
use ryo_values::domain::{
    plan_parts::types::{
        ColumnIndex, Input as PlanInput, JoinColumn, RepeatedRows, Side, StepId, StepScope,
    },
    plans::types::{Emit, Operation, Plan, PlanStep},
    rows::types::Table,
};
use std::sync::Arc;

#[derive(Clone, Copy)]
pub(super) enum ProjectionOrder {
    Forward,
    Reversed,
}
pub(super) fn shared_plan(order: ProjectionOrder, outcome: FinalConsumer) -> Plan {
    let swapped = matches!(order, ProjectionOrder::Reversed);
    let invalid_terminal = matches!(outcome, FinalConsumer::InvalidTerminal);
    let refuse_provider = matches!(outcome, FinalConsumer::RefusingProvider);
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
pub(super) fn numbers(table: &Table) -> Vec<Integer> {
    table
        .cells
        .iter()
        .map(|value| match value {
            Cell::Number(number) => *number,
            other => panic!("number output required, got {other:?}"),
        })
        .collect()
}
#[derive(Clone, Copy)]
pub(super) enum FinalConsumer {
    Completes,
    SkippedAtStart,
    InvalidTerminal,
    RefusingProvider,
}

pub(super) fn exercise(final_consumer: FinalConsumer) -> Result<(), OperatingError> {
    let skipped_at_start = matches!(final_consumer, FinalConsumer::SkippedAtStart);
    let refuse_provider = matches!(final_consumer, FinalConsumer::RefusingProvider);
    let derived = ryo_execution_store::application::file_dag::fixtures::program_derived_port();
    let mut second = rule();
    second.id = RuleId("second-derived-consumer".into());
    let prepared = crate::application::file_dag::program::preparation::service::prepare(
        Compiled {
            schedule: RuleSchedule::Independent,
            observations: vec![vec![], vec![]],
            plans: vec![
                shared_plan(ProjectionOrder::Forward, FinalConsumer::Completes),
                shared_plan(ProjectionOrder::Reversed, final_consumer),
            ],
            bindings: vec![vec![], vec![]],
        },
        &[rule(), second],
        &[Digest([0; 16]), Digest([1; 16])],
        &[],
        Arc::clone(&derived),
        ObservationIdentity::Independent,
    )?;
    let shared = |rule: RuleIndex| {
        let logical = prepared.program().rules()[rule.0].original_steps[1]
            .as_ref()
            .unwrap()
            .logical;
        prepared.program().logical_regions()[logical.0]
    };
    let producer = shared(RuleIndex(0));
    assert_eq!(
        producer,
        shared(RuleIndex(1)),
        "both joins must demand the same actual computed producer"
    );
    assert_eq!(prepared.program().regions()[producer.0].consumers().0, 2);
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(std::sync::Arc::new(
            crate::application::file_dag::fixtures::operators(),
        ))),
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("shared-producer.input".into()),
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
                FileRuleIntent::Skipped(ryo_execution_domain::domain::rule_intent::types::ApplicabilityReason::DiagnosticNotApplicable)
            } else {
                FileRuleIntent::Run {
                    diagnostic: DiagnosticDelivery::Collected,
                    observations: ObservationDemand::NotRequired,
                    capture: CaptureIntent::Disabled,
                }
            },
        ],
        services,
        ryo_values::domain::files::types::SourceView(b""),
        ReuseEligibility::Eligible,
        &mut symbols,
        None,
        None,
    )?;
    // Required views still have to live until their published producer's last
    // consumer when optional retention is unavailable.
    file.owner.derived().set_optional_limit(
        ryo_execution_store::application::file_dag::derived::owner::index::types::ByteLimit(0),
    )?;
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
    use crate::application::file_dag::program::runtime::types::ConsumerReleasePoint;
    let consumer = |rule: RuleIndex, step: StepId| {
        let logical = prepared.program().rules()[rule.0].original_steps[step.0 as usize]
            .as_ref()
            .unwrap()
            .logical;
        prepared.program().logical_regions()[logical.0]
    };
    let first_consumer = consumer(RuleIndex(0), StepId(2));
    let final_consumer_region = consumer(RuleIndex(1), StepId(if refuse_provider { 3 } else { 2 }));
    file.execution.observe_producer_releases(
        file.execution.file_state(),
        producer,
        &[first_consumer, final_consumer_region],
    )?;
    let first = if skipped_at_start {
        assert_eq!(
            file.execution
                .retry_checkpoint(file.execution.file_state(), producer)?
                .remaining
                .0,
            1
        );
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
    assert_eq!(
        numbers(&first),
        [Integer(2), Integer(9), Integer(3), Integer(7)]
    );
    // Delivery is after canonical scheduling. The live interval is observed at
    // the actual consumer release, never inferred from API call order.
    let first_counts = file.owner.derived().counts();
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
                        RootOutcome::Failed(OperatingError::Table(RowsError::NoSuchColumn {
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
                panic!("upfront exclusion is handled before execution")
            }
            FinalConsumer::Completes => {
                let RootOutcome::Complete(second) = second.emit else {
                    return Err(OperatingError::NotRun);
                };
                assert_eq!(
                    numbers(&second),
                    [Integer(9), Integer(2), Integer(7), Integer(3)]
                );
                let final_counts = file.owner.derived().counts();
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
    let events = file
        .execution
        .take_producer_releases(file.execution.file_state());
    assert_eq!(events.len(), if skipped_at_start { 2 } else { 4 });
    let mut live_before = 0;
    let mut weak: Option<&BackingWitness> = None;
    for pair in events.chunks_exact(2) {
        let (before, after) = (&pair[0], &pair[1]);
        assert_eq!(before.point, ConsumerReleasePoint::Before);
        assert_eq!(after.point, ConsumerReleasePoint::After);
        assert_eq!(before.consumer, after.consumer);
        assert_eq!(
            before.checkpoint.remaining.0,
            after.checkpoint.remaining.0 + 1
        );
        if before.index_liveness == IndexLiveness::Alive {
            live_before += 1;
            assert_eq!(before.checkpoint.attempt, AttemptCheckpoint::Published);
            assert!(before.working_bytes.0 > 0);
            let backing = before
                .index
                .as_ref()
                .expect("real acquired equality backing");
            if let Some(first) = weak {
                assert!(first.compare(backing) == BackingIdentity::Same);
            } else {
                weak = Some(backing);
            }
            if after.checkpoint.remaining.0 > 0 {
                assert_eq!(after.checkpoint.attempt, AttemptCheckpoint::Published);
                assert!(
                    after.index_liveness == IndexLiveness::Alive,
                    "nonfinal actual consumption keeps backing live"
                );
                assert!(after.working_bytes.0 > 0);
            }
        }
        if after.checkpoint.remaining.0 == 0 {
            assert_eq!(after.checkpoint.attempt, AttemptCheckpoint::Terminal);
            assert!(after.index_liveness == IndexLiveness::Released);
            assert_eq!(
                after.working_bytes.0, 0,
                "actual final release precedes output delivery and file finish"
            );
        }
    }
    // The invalid Project cannot fuse: canonical demand first completes its
    // actual Join reader, then refuses the Project's output column. Both readers
    // must therefore release at their real invocation boundaries.
    assert_eq!(live_before, if skipped_at_start { 1 } else { 2 });
    let weak = weak.expect("actual healthy consumer held the equality backing");
    let final_state = file
        .execution
        .retry_checkpoint(file.execution.file_state(), producer)?;
    assert_eq!(final_state.attempt, AttemptCheckpoint::Terminal);
    assert_eq!(final_state.remaining.0, 0);
    assert_eq!(
        file.owner.derived().working_bytes(),
        0,
        "final consumption or cancellation releases required storage before file shutdown"
    );
    assert_eq!(
        numbers(&first),
        [Integer(2), Integer(9), Integer(3), Integer(7)],
        "transferred output survives producer retirement"
    );
    assert!(
        weak.liveness() == BackingLiveness::Released,
        "last actual consumer releases the unretained physical index"
    );
    assert_eq!(file.owner.derived().owned_bytes(), 0);
    file.finish()?;
    Ok(())
}
