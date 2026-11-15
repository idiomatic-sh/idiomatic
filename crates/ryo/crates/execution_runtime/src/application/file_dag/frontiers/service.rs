//! The `frontiers` service: what the files hand over to a rule's steps after the files. Each entry of the rule's frontier is
//! read from the store and proven against the files the rule read, in frontier order, before any step runs (`prove`); then
//! read again and decoded once into the phase's symbols (`decode`). A store refusal stops the occurrence; an entry whose producer is unavailable, whose every source
//! was skipped or whose words do not decode is withheld, and its reason is raised where a step first reads it.
use super::constants::{PRODUCER_UNAVAILABLE, PROOF_MISMATCH, SOURCES_SKIPPED};
use super::errors::FrontiersError;
use super::types::{
    FileAccounting, FrontierRows, HandedOver, HandoverPolicy, InputReadiness, PhaseScoping,
    ProvenEntry, ProvenFrontier, ProvenWords, Withholdings,
};
use crate::application::file_dag::program::admission::types::RequiredPartitions;
use crate::application::file_dag::program::preparation::types::PreparedProgram;
use crate::application::file_dag::program::request_execution::types::RequestExecution;
use crate::application::file_dag::project_source::service::table_of;
use crate::application::file_dag::project_source::types::ProjectSource;
use crate::outbound::facts::interface::FactsFinish;
use crate::outbound::facts::types::KeptBucket;
use ryo_execution_domain::domain::completion::service::partial_of_lacks;
use ryo_execution_domain::domain::completion::types::{CoverageState, FileLacks, MissingRows};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::frontier_coverage::service::restored_lacks;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_language::domain::symbols::service::places_mut;
use ryo_language::domain::symbols::types::Symbols;
use ryo_operators::domain::contract::types::WithheldInput;
use ryo_operators::domain::physical_plan::types::frontier_place::value::FrontierPlace;
use ryo_operators::domain::physical_plan::types::held_rows::value::HeldRows;
use ryo_relations::domain::catalog::service::scope_of;
use ryo_relations::domain::catalog::types::RowScope;
use ryo_relations::domain::relations::types::FileId;
use ryo_values::domain::plan_parts::types::{Input, InputNumber};
use ryo_values::domain::records::service::is_well_formed;
use ryo_values::domain::records::types::{FrontierCoverage, FrontierShape};
use ryo_values::domain::rows::types::Table;
use ryo_values::domain::run_metadata::types::FrontierFeed;
use ryo_values::domain::stored_facts::service::frontier_bucket;
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::table_words::service::{
    decode_combined, decode_complete, single_table_frame,
};
use ryo_values::domain::table_words::types::{Framing, PackedSlice};
use std::collections::BTreeSet;
use std::sync::Arc;

/// The files `rule` reads after the files: the request's files, less those it skipped, does not apply to or was refused on.
///
/// # Errors
///
/// `NotRun` for a rule outside the program.
pub(crate) fn required_files(
    program: &PreparedProgram,
    execution: &RequestExecution,
    rule: RuleIndex,
    scoping: PhaseScoping<'_>,
) -> Result<RequiredPartitions, OperatingError> {
    let _rule = program.rule_plan(rule)?;
    RequiredPartitions::from_request(
        execution.services.request(),
        scoping.corpus,
        scoping.covered,
        scoping.skipped,
        scoping
            .inapplicable
            .get(rule.0)
            .map_or(&[][..], |files| &files.0),
        scoping
            .refused
            .get(rule.0)
            .map_or(&[][..], |files| &files.0),
    )
}

/// What the store keeps of `rule`'s frontier, by entry: each entry the files keep read from the store and proven in frontier
/// order, its words dropped once proven. An entry a project relation answers is the phase's table, and one a bound observation
/// answers is not the files' to hand over.
///
/// # Errors
///
/// The store's refusal of a bucket, or a proof that does not fit the program: the occurrence stops before any step runs.
pub(crate) fn prove(
    program: &PreparedProgram,
    rule: RuleIndex,
    first: Bucket,
    facts: &dyn FactsFinish,
    required: &RequiredPartitions,
    policy: HandoverPolicy<'_>,
    project: &ProjectSource<'_>,
) -> Result<ProvenFrontier, FrontiersError> {
    let frontier = program.frontiers(rule)?;
    let feeds = &program
        .metadata()
        .get(rule.0)
        .ok_or(OperatingError::NotRun)?
        .frontier_capture;
    let mut proven = Vec::with_capacity(frontier.len());
    for (place, input) in frontier.iter().copied().enumerate() {
        if let HandoverPolicy::Observations { ready } = policy
            && ready.0.get(place).copied().ok_or(OperatingError::NotRun)?
                == InputReadiness::Unavailable
        {
            proven.push(ProvenEntry::Refused(
                OperatingError::ObservationIncomplete {
                    reason: PRODUCER_UNAVAILABLE.into(),
                },
            ));
            continue;
        }
        if feeds.get(place).copied().ok_or(OperatingError::NotRun)? == FrontierFeed::CrossFile {
            proven.push(ProvenEntry::Elsewhere);
            continue;
        }
        // A project relation is the project's to hand over, read once for the phase.
        if let Input::Relation(relation) = input
            && scope_of(relation) == RowScope::Project
        {
            proven.push(ProvenEntry::Project(Arc::clone(table_of(
                project, relation,
            )?)));
            continue;
        }
        let ProvenWords { words, coverage } =
            kept(program, rule, first, InputNumber(place), facts, required)?;
        if words.sources.is_empty() && !required.files().is_empty() {
            proven.push(ProvenEntry::Refused(
                OperatingError::ObservationIncomplete {
                    reason: SOURCES_SKIPPED.into(),
                },
            ));
            continue;
        }
        proven.push(ProvenEntry::Words { coverage });
    }
    Ok(ProvenFrontier(proven))
}

/// What the files hand over to `rule`'s steps after the files, by frontier entry, from what `prove` left of it: each kept
/// entry's words read from the store again, proven again and decoded into `symbols`; a refused one withheld with its reason.
///
/// # Errors
///
/// The store's refusal of a bucket, or a proof that no longer fits the program: the occurrence stops before any step runs.
#[expect(
    clippy::too_many_arguments,
    reason = "the proof's own inputs, read again, with the phase's symbols and withholdings it decodes into"
)]
pub(crate) fn decode(
    program: &PreparedProgram,
    rule: RuleIndex,
    first: Bucket,
    facts: &dyn FactsFinish,
    symbols: &mut Symbols,
    required: &RequiredPartitions,
    policy: HandoverPolicy<'_>,
    proven: &ProvenFrontier,
    withholdings: &mut Withholdings,
) -> Result<HandedOver, FrontiersError> {
    let mut handed = Vec::with_capacity(proven.0.len());
    for (place, entry) in proven.0.iter().enumerate() {
        handed.push(match entry {
            ProvenEntry::Elsewhere => FrontierRows::Elsewhere,
            ProvenEntry::Project(table) => FrontierRows::Held {
                table: Arc::new(Table::clone(table)),
                coverage: CoverageState::Complete,
            },
            ProvenEntry::Refused(error) => {
                FrontierRows::Withheld(withheld(withholdings, error.clone()))
            }
            ProvenEntry::Words { coverage } => {
                let ProvenWords { words, .. } =
                    kept(program, rule, first, InputNumber(place), facts, required)?;
                match decoded(&words, symbols, policy) {
                    Ok(table) => FrontierRows::Held {
                        table: Arc::new(table),
                        coverage: coverage.clone(),
                    },
                    Err(error) => FrontierRows::Withheld(withheld(withholdings, error)),
                }
            }
        });
    }
    Ok(HandedOver(handed))
}

/// The words the files kept for frontier entry `input` of `rule`, read from the store, and the coverage their proof gives.
///
/// # Errors
///
/// The store's refusal of the bucket, or a proof that does not fit the program.
fn kept(
    program: &PreparedProgram,
    rule: RuleIndex,
    first: Bucket,
    input: InputNumber,
    facts: &dyn FactsFinish,
    required: &RequiredPartitions,
) -> Result<ProvenWords, FrontiersError> {
    let bucket = frontier_bucket(first, input);
    let words = facts.kept(bucket)?;
    let coverage = validate_bucket(program, rule, input, first, bucket, required, &words)?;
    Ok(ProvenWords { words, coverage })
}

/// The rows of `words` in `symbols`: for the observations every row read and the first malformed one refused, for the
/// diagnostic what reads.
fn decoded(
    words: &KeptBucket,
    symbols: &mut Symbols,
    policy: HandoverPolicy<'_>,
) -> Result<Table, OperatingError> {
    match policy {
        HandoverPolicy::Observations { .. } => decode_complete(&words.words.0, places_mut(symbols))
            .map_err(|error| OperatingError::ObservationIncomplete {
                reason: error.to_string(),
            }),
        HandoverPolicy::Diagnostic => Ok(decode_combined(&words.words.0, places_mut(symbols))),
    }
}

/// The rows the files handed over for `place`, or the token of why they are withheld.
pub(crate) fn held_frontier(handed: &HandedOver, place: FrontierPlace) -> HeldRows<'_> {
    match handed.0.get(place.input.0) {
        Some(FrontierRows::Held { table, .. }) => HeldRows::Held(table),
        Some(FrontierRows::Withheld(token)) => HeldRows::Withheld(*token),
        Some(FrontierRows::Elsewhere) | None => HeldRows::Withheld(WithheldInput(u32::MAX)),
    }
}

/// The coverage of the rows the files handed over for entry `input`, when they are held.
pub(crate) fn handed_coverage(handed: &HandedOver, input: InputNumber) -> Option<&CoverageState> {
    match handed.0.get(input.0) {
        Some(FrontierRows::Held { coverage, .. }) => Some(coverage),
        Some(FrontierRows::Withheld(_) | FrontierRows::Elsewhere) | None => None,
    }
}

/// Records why an input is withheld and answers its token.
pub(crate) fn withheld(withholdings: &mut Withholdings, error: OperatingError) -> WithheldInput {
    let token = WithheldInput(u32::try_from(withholdings.0.len()).unwrap_or(u32::MAX));
    withholdings.0.push(error);
    token
}

/// Why the input of `token` was withheld.
pub(crate) fn reason(withholdings: &Withholdings, token: WithheldInput) -> OperatingError {
    match usize::try_from(token.0) {
        Ok(at) => withholdings
            .0
            .get(at)
            .cloned()
            .unwrap_or(OperatingError::NotRun),
        Err(_) => OperatingError::NotRun,
    }
}

/// Checks every range and proof of a kept bucket before its rows are read: each frame the rule's own, whole and of a required
/// file, every required file accounted for. The file number comes from the run's handover; a stored owner is never trusted.
///
/// # Errors
///
/// `PROOF_MISMATCH` when the bucket does not fit the program or the files.
pub(crate) fn validate_bucket(
    program: &PreparedProgram,
    rule: RuleIndex,
    input: InputNumber,
    first: Bucket,
    bucket: Bucket,
    required: &RequiredPartitions,
    payload: &KeptBucket,
) -> Result<CoverageState, OperatingError> {
    let refuse = || OperatingError::ObservationIncomplete {
        reason: PROOF_MISMATCH.into(),
    };
    let expected = program.rule_digest(rule)?;
    let skipped = required.skipped_files(program, rule)?;
    if frontier_bucket(first, input) != bucket || program.frontiers(rule)?.get(input.0).is_none() {
        return Err(refuse());
    }
    let mut at = 0;
    let mut seen_files = BTreeSet::new();
    let mut affected = Vec::new();
    for source in &payload.sources {
        if source.start.0 != at
            || source.length.0 == 0
            || !required.files().contains(&source.file)
            || skipped.contains(&source.file)
            || !seen_files.insert(source.file)
            || source.proof.rule.0 != u32::try_from(rule.0).map_err(|_| refuse())?
            || source.proof.input.0 != u32::try_from(input.0).map_err(|_| refuse())?
            || source.proof.rule_digest != expected
        {
            return Err(refuse());
        }
        let end = at.checked_add(source.length.0).ok_or_else(refuse)?;
        let frame = payload.words.0.0.get(at..end).ok_or_else(refuse)?;
        if single_table_frame(PackedSlice(frame)) != Framing::Whole {
            return Err(refuse());
        }
        at = end;
        match &source.proof.coverage {
            FrontierCoverage::Complete => {}
            FrontierCoverage::Partial(scopes) => {
                if is_well_formed(scopes) == FrontierShape::Malformed {
                    return Err(refuse());
                }
                affected.extend(restored_lacks(source.file, scopes));
            }
        }
    }
    // A file the rule was refused on has no frame: one that carries a frame contradicts the edge.
    let refused = required
        .refused()
        .iter()
        .filter(|file| required.files().contains(file))
        .copied()
        .collect::<BTreeSet<_>>();
    if refused
        .iter()
        .any(|file| seen_files.contains(file) || skipped.contains(file))
    {
        return Err(refuse());
    }
    // Known unavailable files are accounted for without inventing empty frames; they stay attached to the partial answer.
    seen_files.extend(skipped.iter().copied());
    seen_files.extend(refused.iter().copied());
    // A required file with no frame is discharged only when the edge named it inapplicable or refused to this rule.
    seen_files.extend(
        required
            .inapplicable()
            .iter()
            .filter(|file| required.files().contains(file))
            .copied(),
    );
    if at != payload.words.0.0.len()
        || exact_required_files(required.files(), &seen_files) == FileAccounting::Inexact
    {
        return Err(refuse());
    }
    // A refused file lacks the rule's input, so the answer is partial over it, never complete.
    affected.extend(refused.into_iter().map(|file| FileLacks {
        file: Some(file),
        missing: MissingRows::Any,
    }));
    // A skipped file stays unavailable: it lacks input without making an answer not keyed by it partial.
    Ok(if !affected.is_empty() {
        affected.extend(skipped.iter().copied().map(|file| FileLacks {
            file: Some(file),
            missing: MissingRows::Any,
        }));
        partial_of_lacks(affected)
    } else if skipped.is_empty() {
        CoverageState::Complete
    } else {
        CoverageState::Inapplicable {
            files: skipped.into_iter().collect(),
        }
    })
}

/// Whether `seen` names exactly the `required` files.
fn exact_required_files(required: &[FileId], seen: &BTreeSet<FileId>) -> FileAccounting {
    if seen.len() == required.len() && required.iter().all(|file| seen.contains(file)) {
        FileAccounting::Exact
    } else {
        FileAccounting::Inexact
    }
}
