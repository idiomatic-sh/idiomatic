//! The producer lifetime scenario every exit runs.
use super::*;
use crate::application::file_dag::program::facade_runtime::service::ProgramServices;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::CaptureIntent;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::DiagnosticDelivery;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::EmitDemand;
use ryo_execution_domain::domain::rule_intent::types::FileRuleIntent;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::ObservationDemand;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::RuleDemand;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_execution_store::application::file_dag::relations::types::ReuseEligibility;
use std::borrow::Cow;
/// What the second rule's final consumer answered.
#[derive(Debug, PartialEq)]
pub(in super::super) enum Second {
    /// It completed with these numbers.
    Rows(Cow<'static, [i64]>),
    /// It failed with this error.
    Failed(OperatingError),
    /// It was refused before running, being excluded at the start.
    Refused(OperatingError),
}

/// What one delivered or replayed root answered: its rows, or the failure it ended with.
#[derive(Clone, Debug, PartialEq)]
pub(in super::super) enum Delivery {
    Rows(usize),
    Failed(OperatingError),
    /// Any other outcome.
    Lost,
}

impl From<RootOutcome> for Delivery {
    fn from(outcome: RootOutcome) -> Self {
        match outcome {
            RootOutcome::Complete(table) => Self::Rows(table.count.0),
            RootOutcome::Failed(error) => Self::Failed(error),
            RootOutcome::Partial { .. }
            | RootOutcome::Skipped(_)
            | RootOutcome::Unknown(_)
            | RootOutcome::NoQuery
            | RootOutcome::Deferred { .. } => Self::Lost,
        }
    }
}

/// One consumer's release: the remaining consumers, attempt and index liveness before and
/// after it, whether working bytes were held before and after, and whether the before and after
/// events name the same consumer in order.
#[derive(Clone, Debug, PartialEq)]
pub(in super::super) struct Release {
    pub(in super::super) remaining: (usize, usize),
    pub(in super::super) attempt: (AttemptCheckpoint, AttemptCheckpoint),
    pub(in super::super) alive: (bool, bool),
    pub(in super::super) holds_bytes: (bool, bool),
    pub(in super::super) paired: bool,
}

/// What the shared producer's lifetime observed.
#[derive(Debug, PartialEq)]
pub(in super::super) struct Lifetime {
    /// Whether both joins demand one producer region, and that region's consumer count.
    pub(in super::super) shared_consumers: Option<usize>,
    /// The producer's remaining consumers after the start exclusion, when one was excluded.
    pub(in super::super) remaining_after_exclusion: Option<usize>,
    pub(in super::super) first: Cow<'static, [i64]>,
    /// The equality builds after the first consumer, and after the second with whether it hit.
    pub(in super::super) builds: (u64, u64, bool),
    pub(in super::super) second: Second,
    /// The source reads the final consumer added.
    pub(in super::super) final_rereads: usize,
    pub(in super::super) releases: Cow<'static, [Release]>,
    /// Whether every live consumer saw the same equality backing.
    pub(in super::super) one_backing: bool,
    /// The producer's final attempt and remaining consumers.
    pub(in super::super) final_state: (AttemptCheckpoint, usize),
    pub(in super::super) working_bytes: usize,
    /// Whether the transferred first output is unchanged after the producer retired.
    pub(in super::super) first_survives: bool,
    pub(in super::super) backing_released: bool,
    pub(in super::super) owned_bytes: usize,
}

fn integers(table: &Table) -> Cow<'static, [i64]> {
    numbers(table).into_iter().map(|number| number.0).collect()
}

pub(in super::super) fn exercise(
    final_consumer: FinalConsumer,
) -> Result<Lifetime, OperatingError> {
    let skipped_at_start = matches!(final_consumer, FinalConsumer::SkippedAtStart);
    let refuse_provider = matches!(final_consumer, FinalConsumer::RefusingProvider);
    let derived = ryo_execution_store::application::file_dag::fixtures::program_derived_port();
    let mut second = rule();
    second.id = RuleId("second-derived-consumer".into());
    let prepared = crate::application::file_dag::program::preparation::service::prepare(
        Compiled {
            schedule: RuleSchedule::Independent,
            observations: vec![vec![], vec![]],
            plans: std::iter::IntoIterator::into_iter(vec![
                shared_plan(ProjectionOrder::Forward, FinalConsumer::Completes),
                shared_plan(ProjectionOrder::Reversed, final_consumer),
            ])
            .map(std::sync::Arc::new)
            .collect(),
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
    let shared_consumers = (producer == shared(RuleIndex(1)))
        .then(|| prepared.program().regions()[producer.0].consumers().0);
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(Arc::new(
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
    let mut remaining_after_exclusion = None;
    let first = if skipped_at_start {
        remaining_after_exclusion = Some(
            file.execution
                .retry_checkpoint(file.execution.file_state(), producer)?
                .remaining
                .0,
        );
        let mut cells = vec![];
        // The emission's summary is not this case's observation; the delivered cells are.
        let _emission = file.emit_rows(RuleIndex(0), &read, &mut symbols, &mut |row| {
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
    let first_numbers = integers(&first);
    // Delivery is after canonical scheduling. The live interval is observed at
    // the actual consumer release, never inferred from API call order.
    let first_counts = file.owner.derived().counts();
    let reads_before_final = source_reads.get();
    let second = if skipped_at_start {
        match file.rule(RuleIndex(1), demand(), &read, &mut symbols).err() {
            Some(error) => Second::Refused(error),
            None => return Err(OperatingError::NotRun),
        }
    } else {
        match file.rule(RuleIndex(1), demand(), &read, &mut symbols)?.emit {
            RootOutcome::Complete(second) => Second::Rows(integers(&second)),
            RootOutcome::Failed(error) => Second::Failed(error),
            RootOutcome::Partial { .. }
            | RootOutcome::Skipped(_)
            | RootOutcome::Unknown(_)
            | RootOutcome::NoQuery
            | RootOutcome::Deferred { .. } => return Err(OperatingError::NotRun),
        }
    };
    let final_counts = file.owner.derived().counts();
    let final_rereads = source_reads.get() - reads_before_final;
    let events = file
        .execution
        .take_producer_releases(file.execution.file_state());
    let mut releases = Vec::new();
    let mut one_backing = true;
    let mut weak: Option<&BackingWitness> = None;
    for pair in events.chunks_exact(2) {
        let (before, after) = (&pair[0], &pair[1]);
        if let Some(backing) = before.index.as_ref() {
            match weak {
                Some(first) => one_backing &= first.compare(backing) == BackingIdentity::Same,
                None => weak = Some(backing),
            }
        }
        releases.push(Release {
            remaining: (before.checkpoint.remaining.0, after.checkpoint.remaining.0),
            attempt: (before.checkpoint.attempt, after.checkpoint.attempt),
            alive: (
                before.index_liveness == IndexLiveness::Alive,
                after.index_liveness == IndexLiveness::Alive,
            ),
            holds_bytes: (before.working_bytes.0 > 0, after.working_bytes.0 > 0),
            paired: before.point == ConsumerReleasePoint::Before
                && after.point == ConsumerReleasePoint::After
                && before.consumer == after.consumer,
        });
    }
    let backing_released = weak.is_some_and(|weak| weak.liveness() == BackingLiveness::Released);
    let final_state = file
        .execution
        .retry_checkpoint(file.execution.file_state(), producer)?;
    let lifetime = Lifetime {
        shared_consumers,
        remaining_after_exclusion,
        builds: (first_counts.0.0, final_counts.0.0, final_counts.1.0 > 0),
        second,
        final_rereads,
        releases: releases.into(),
        one_backing,
        final_state: (final_state.attempt, final_state.remaining.0),
        working_bytes: file.owner.derived().working_bytes().0,
        first_survives: integers(&first) == first_numbers,
        first: first_numbers,
        backing_released,
        owned_bytes: file.owner.derived().owned_bytes().0,
    };
    // The finished file's frontier readiness is not this case's observation.
    let _readiness = file.finish()?;
    Ok(lifetime)
}
