//! The `reading` service: free functions named by verb over the visit of one file, with no IO and no ports.

use super::types::{EngineAsked, EngineRows};
use crate::domain::coverage::service::partial_of;
use crate::domain::coverage::types::Recovery;
use crate::domain::coverage::types::RuleIndex;
use crate::domain::finish::service::escape;
use crate::domain::finish::types::{Arena, Leaves, Leaving, Retained, RuleNumber};
use crate::domain::operating::types::OperatorContext;
use crate::domain::operator::service::{emitted, pool_for};
use crate::domain::operator::types::RuleWorkspace;
use crate::domain::ordering::service::sort_findings;
use crate::domain::placement::service::governs;
use crate::domain::placement::types::{Governing, Mask};
use crate::domain::render::service::{render_rows, rendered_limit, shown_by};
use crate::domain::render::types::{Answering, FindingRule, RenderInput};
use crate::domain::rules::errors::RulesError;
use crate::domain::rules::service::{failed, id_at};
use crate::domain::table_cells::types::{TreeForm, TreeRows};

use crate::domain::table_texts::types::SourceView;
use crate::domain::table_words::service::{encode_table, pack_onto};
use crate::domain::table_words::types::WordSlice;
use crate::domain::trust::types::Needs;
use crate::domain::value_cells::types::Scene;
use crate::domain::workers::types::{Tables, VisitSetting};
use ryo_language::domain::elements::service::clear_elements;
use ryo_relations::domain::relations::types::{ColumnsView, FileId, Relation};
use ryo_values::domain::plan_parts::types::InputNumber;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::records::types::Kept;
use ryo_values::domain::records::types::PlacedRule;
use ryo_values::domain::stored_facts::types::{ArenaBase, Bucket, KeptWords, ReadInPart};

/// The bucket the rows of the frontier's input at `input` of a rule wait in, the rule's first bucket being `first`.
#[must_use]
pub(crate) fn frontier_bucket(first: Bucket, input: InputNumber) -> Bucket {
    Bucket(
        first
            .0
            .saturating_add(u32::try_from(input.0).unwrap_or(u32::MAX)),
    )
}

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
    wanted: &[Relation],
    mask: &Mask,
    needs: &[Needs],
    rule_count: usize,
) -> bool {
    if !wanted.contains(&Relation::FilesystemReadFailure) {
        return false;
    }
    if rule_count == 0 || needs.len() != rule_count {
        return true;
    }
    needs.iter().enumerate().any(|(place, rule)| {
        governs(mask, RuleNumber(u32::try_from(place).unwrap_or(u32::MAX))) == Governing::Governs
            && rule
                .reads
                .iter()
                .any(|read| read.relation == Relation::FilesystemReadFailure)
    })
}

/// Readiness after a primary input could not be admitted: source-derived facts are unknown,
/// while independently observed engine metadata and the typed failure event remain decidable.
pub(crate) fn failure_partials(
    wanted: &[Relation],
    file: FileId,
    detail: &str,
    observed_file: bool,
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
            ) && (!observed_file || **relation != Relation::FilesystemFile)
        })
        .map(|relation| ReadInPart {
            relation: *relation,
            cause: PartialCause(detail.into()),
            lacking: ryo_relations::domain::relations::types::Missing::Any,
            at: PartialScope::OneFile(PartialAt(file.0)),
        })
        .collect()
}

/// Notes the relations the file is read in part: those a reader said so of, and all of code if what was read is a recovery.
pub(crate) fn start_partials(
    tables: &mut Tables,
    said: &[ReadInPart],
    everything: Recovery,
    wanted: &[Relation],
) {
    tables.partials = partial_of(said, everything, wanted);
}

/// Hands what the rule's per-file steps leave for the cross-file steps to `keep`, input by input, and keeps a copy when the run
/// keeps its findings.
///
/// # Errors
///
/// Whatever `keep` fails with.
pub(crate) fn prepare_frontier(
    tables: &mut Tables,
    setting: &VisitSetting<'_>,
    rule: RuleIndex,
    file: FileId,
    place: InputNumber,
    frontier: &mut [Table],
) -> Option<Bucket> {
    if !*setting
        .metadata
        .get(rule.0)?
        .frontier_capture
        .get(place.0)?
    {
        return None;
    }
    let first = setting.first_bucket.get(rule.0).copied()?;
    let table = frontier.get_mut(place.0)?;
    escape(
        table,
        Leaving {
            symbols: &tables.symbols,
            source: SourceView(&tables.bytes.0),
            file,
            worker: tables.number,
            text: Leaves::WithText,
        },
        Retained {
            arena: Arena(&mut tables.arena.0),
            places: &mut tables.places,
        },
    );
    tables.words.0.clear();
    encode_table(table, &tables.places, &mut tables.words);
    tables.places.clear();
    tables.packed.0.clear();
    pack_onto(WordSlice(&tables.words.0), &mut tables.packed);
    Some(frontier_bucket(first, place))
}

/// Capture the prepared words only after the caller successfully retained them.
pub(crate) fn capture_frontier(
    tables: &mut Tables,
    answering: &Answering,
    bucket: Bucket,
    proof: ryo_values::domain::records::types::FrontierProof,
) {
    if *answering == Answering::Findings {
        tables.captured.push(Kept {
            bucket,
            proof,
            words: KeptWords(tables.packed.clone()),
        });
    }
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
    metadata: &[crate::domain::run_metadata::types::RuleMetadata],
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

use ryo_values::domain::rows::types::Table;
