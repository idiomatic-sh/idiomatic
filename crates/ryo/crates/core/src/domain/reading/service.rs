//! The `reading` service: free functions named by verb over the visit of one file, with no IO and no ports.

use super::types::{
    EngineAsked, EngineRows, FailureDetail, ObservedFile, ReadFailureDemand, RunRuleCount,
};
use crate::domain::ordering::service::sort_findings;
use crate::domain::render::types::Answering;
use ryo_execution::inbound::use_cases::metadata::interface::MetadataUseCase;
use ryo_execution::inbound::use_cases::metadata::interface::Recovery;
use ryo_execution::inbound::use_cases::metadata::interface::{Governing, Mask};

use crate::domain::workers::types::Tables;
use ryo_execution::inbound::use_cases::metadata::interface::Needs;
use ryo_language::domain::elements::service::clear_elements;
use ryo_relations::domain::relations::types::{FileId, Relation};
use ryo_values::domain::plan_parts::types::InputNumber;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::row_texts::types::PlacedRule;
use ryo_values::domain::stored_facts::types::{ArenaBase, Bucket, ReadInPart};

/// Readies the worker for a file it reads: counts it, notes where its text will begin in the worker's arena and empties what the
/// last file left.
pub(crate) fn begin(tables: &mut Tables) -> ArenaBase {
    tables.reads.0 += 1;
    prepare(tables)
}

/// Resets per-file language state before execution without claiming a successful source read.
pub(crate) fn prepare(tables: &mut Tables) -> ArenaBase {
    let base = ArenaBase(u32::try_from(tables.arena.0.len()).unwrap_or(u32::MAX));
    tables.captured.clear();
    tables.current_observations = None;
    clear_elements(&mut tables.elements);
    base
}

/// Which rows the engine writes of every file the relations asked for include.
#[must_use]
pub(crate) fn engine_asked(wanted: &[Relation]) -> EngineAsked {
    let asked = |relation: Relation| {
        if wanted.contains(&relation) {
            EngineRows::Asked
        } else {
            EngineRows::NotAsked
        }
    };
    let file = asked(Relation::FilesystemFile);
    let lines = asked(Relation::FilesystemLineCount);
    let executable = asked(Relation::FilesystemExecutable);
    let read_failure = asked(Relation::FilesystemReadFailure);
    let any = if file == EngineRows::Asked
        || lines == EngineRows::Asked
        || executable == EngineRows::Asked
        || read_failure == EngineRows::Asked
    {
        EngineRows::Asked
    } else {
        EngineRows::NotAsked
    };
    EngineAsked {
        file,
        lines,
        executable,
        read_failure,
        any,
    }
}

/// Whether a rule governing this file reads the primary disk-failure observation. Incomplete
/// legacy rule metadata keeps the conservative run-wide demand.
#[must_use]
pub(crate) fn read_failure_demanded(
    metadata: &impl MetadataUseCase,
    wanted: &[Relation],
    mask: &Mask,
    needs: &[Needs],
    rule_count: RunRuleCount,
) -> ReadFailureDemand {
    if !wanted.contains(&Relation::FilesystemReadFailure) {
        return ReadFailureDemand::NotDemanded;
    }
    if rule_count.0 == 0 || needs.len() != rule_count.0 {
        return ReadFailureDemand::Demanded;
    }
    let reads = needs.iter().enumerate().any(|(place, rule)| {
        metadata.governs(mask, RuleNumber(u32::try_from(place).unwrap_or(u32::MAX)))
            == Governing::Governs
            && rule
                .reads
                .iter()
                .any(|read| read.relation == Relation::FilesystemReadFailure)
    });
    if reads {
        ReadFailureDemand::Demanded
    } else {
        ReadFailureDemand::NotDemanded
    }
}

/// Readiness after a primary input could not be admitted: source-derived facts are unknown,
/// while independently observed engine metadata and the typed failure event remain decidable.
pub(crate) fn failure_partials(
    wanted: &[Relation],
    file: FileId,
    detail: FailureDetail<'_>,
    observed_file: ObservedFile,
) -> Vec<ryo_values::domain::stored_facts::types::ReadInPart> {
    use ryo_values::domain::stored_facts::types::{
        PartialAt, PartialCause, PartialScope, ReadInPart,
    };
    wanted
        .iter()
        .filter(|relation| {
            !matches!(
                relation,
                Relation::FilesystemReadFailure | Relation::FilesystemExecutable
            ) && (observed_file == ObservedFile::Unobserved || **relation != Relation::FilesystemFile)
        })
        .map(|relation| ReadInPart {
            relation: *relation,
            cause: PartialCause(detail.0.into()),
            lacking: ryo_relations::domain::relations::types::Missing::Any,
            at: PartialScope::OneFile(PartialAt(file.0)),
        })
        .collect()
}

/// Notes the relations the file is read in part: those a reader said so of, and all of code if what was read is a recovery.
pub(crate) fn start_partials(
    metadata: &impl MetadataUseCase,
    tables: &mut Tables,
    said: &[ReadInPart],
    everything: Recovery,
    wanted: &[Relation],
) {
    tables.partials = metadata.partial_of(said, everything, wanted);
}

/// Gives back what a huge file made the tables grow past, and puts the file's findings in their order: rows of the rules' own
/// columns stay in the order of the rules and, for a rule, of their places.
pub(crate) fn settle(tables: &mut Tables, answering: &Answering) {
    if *answering == Answering::Findings {
        sort_findings(&mut tables.rendered);
    }
}

/// The buckets of the frontiers of the rules `rules`, by their place among `plans`, in order: every bucket when `rules` is all.
#[must_use]
pub(crate) fn buckets_of(
    metadata: &[ryo_values::domain::run_metadata::types::RuleRunMetadata],
    first: &[Bucket],
    rules: Option<&[PlacedRule]>,
) -> Vec<Bucket> {
    let mut buckets = Vec::new();
    for (place, (rule, start)) in metadata.iter().zip(first).enumerate() {
        let rule = PlacedRule(u32::try_from(place).unwrap_or(u32::MAX));
        if rules.is_some_and(|rules| rules.binary_search_by_key(&rule.0, |each| each.0).is_err()) {
            continue;
        }
        for input in 0..metadata[place].frontier_capture.len() {
            buckets.push(frontier_bucket(*start, InputNumber(input)));
        }
    }
    buckets.sort_unstable();
    buckets.dedup();
    buckets
}

/// The first bucket of each rule's frontier: the findings have bucket zero.
#[must_use]
pub(crate) fn first_buckets(plans: &[Plan]) -> Vec<Bucket> {
    let mut first = Vec::with_capacity(plans.len());
    let mut next = Bucket(1);
    for plan in plans {
        first.push(next);
        next = Bucket(
            next.0
                .saturating_add(u32::try_from(plan.frontier.len()).unwrap_or(u32::MAX)),
        );
    }
    first
}

use ryo_values::domain::stored_facts::service::frontier_bucket;

use ryo_values::domain::runs::types::RuleNumber;
