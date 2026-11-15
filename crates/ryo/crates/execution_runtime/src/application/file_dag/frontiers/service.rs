//! The `frontiers` service: what the files hand over to a rule's steps after the files. Each entry of the rule's frontier is
//! proven from its provenance against the files the rule read, in frontier order, before any step runs (`prove`); then its
//! words are read once, proven with the provenance read with them and decoded into the phase's symbols (`decode`). A store refusal stops the occurrence; an entry whose producer is unavailable, whose every source
//! was skipped or whose words do not decode is withheld, and its reason is raised where a step first reads it.
use super::constants::{PRODUCER_UNAVAILABLE, SOURCES_SKIPPED};
use super::errors::FrontiersError;
use super::types::{
    BucketPlace, BucketProvenance, FrontierRows, HandedOver, HandoverPolicy, InputReadiness,
    PhaseScoping, ProvenEntry, ProvenFrontier, ReadinessProof, Withholdings,
};
use crate::application::file_dag::program::admission::types::RequiredPartitions;
use crate::application::file_dag::program::preparation::types::PreparedProgram;
use crate::application::file_dag::program::request_execution::types::RequestExecution;
use crate::application::file_dag::project_source::service::table_of;
use crate::application::file_dag::project_source::types::ProjectSource;
use crate::outbound::facts::interface::FactsFinish;
use crate::outbound::facts::types::{BucketSource, KeptBucket};
use ryo_execution_domain::domain::completion::types::CoverageState;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_language::domain::symbols::service::places_mut;
use ryo_language::domain::symbols::types::Symbols;
use ryo_operators::domain::contract::types::WithheldInput;
use ryo_operators::domain::physical_plan::types::frontier_place::value::FrontierPlace;
use ryo_operators::domain::physical_plan::types::held_rows::value::HeldRows;
use ryo_relations::domain::catalog::service::scope_of;
use ryo_relations::domain::catalog::types::RowScope;
use ryo_values::domain::plan_parts::types::{Input, InputNumber};
use ryo_values::domain::rows::types::Table;
use ryo_values::domain::run_metadata::types::FrontierFeed;
use ryo_values::domain::stored_facts::service::frontier_bucket;
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::table_words::service::{decode_combined, decode_complete};
use std::sync::Arc;

pub(crate) mod validation;
use validation::service::{validate_bucket, validate_sources};

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
        // The proof reads the bucket's provenance alone: its words are read once, where the entry is decoded.
        let bucket = frontier_bucket(first, InputNumber(place));
        let kept = facts.sources(bucket)?;
        let _proved = validate_sources(
            program,
            BucketPlace {
                rule,
                input: InputNumber(place),
                first,
                bucket,
            },
            required,
            BucketProvenance {
                sources: &kept.sources,
                length: kept.length,
            },
        )?;
        proven.push(match every_source_skipped(&kept.sources, required) {
            Some(refusal) => ProvenEntry::Refused(refusal),
            None => ProvenEntry::Words,
        });
    }
    Ok(ProvenFrontier(proven))
}

/// What the files hand over to `rule`'s steps after the files, by frontier entry, from what `prove` left of it: each kept
/// entry's words read from the store once and proven with the provenance read with them, whose coverage they carry; a refused
/// one withheld with its reason.
///
/// # Errors
///
/// The store's refusal of a bucket, or words whose own proof does not fit the program: the occurrence stops before any step
/// runs.
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
                table: Arc::clone(table),
                coverage: CoverageState::Complete,
            },
            ProvenEntry::Refused(error) => {
                FrontierRows::Withheld(withheld(withholdings, error.clone()))
            }
            ProvenEntry::Words => {
                let bucket = frontier_bucket(first, InputNumber(place));
                let words = facts.kept(bucket)?;
                // The words carry the coverage of their own proof, never an earlier read's.
                let coverage = validate_bucket(
                    program,
                    rule,
                    InputNumber(place),
                    first,
                    bucket,
                    required,
                    &words,
                )?;
                match every_source_skipped(&words.sources, required)
                    .map_or_else(|| decoded(&words, symbols, policy), Err)
                {
                    Ok(table) => FrontierRows::Held {
                        table: Arc::new(table),
                        coverage,
                    },
                    Err(error) => FrontierRows::Withheld(withheld(withholdings, error)),
                }
            }
        });
    }
    Ok(HandedOver(handed))
}

/// What a frontier is handed over for: the observations with each entry's readiness, or the diagnostic when there is none.
pub(crate) fn policy_of(ready: Option<&ReadinessProof>) -> HandoverPolicy<'_> {
    match ready {
        Some(ready) => HandoverPolicy::Observations { ready },
        None => HandoverPolicy::Diagnostic,
    }
}

/// Why an entry is withheld when the files are required but every source of its bucket was skipped, if it is.
fn every_source_skipped(
    sources: &[BucketSource],
    required: &RequiredPartitions,
) -> Option<OperatingError> {
    (sources.is_empty() && !required.files().is_empty()).then(|| {
        OperatingError::ObservationIncomplete {
            reason: SOURCES_SKIPPED.into(),
        }
    })
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
