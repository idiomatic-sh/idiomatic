//! The `project_pass` service: a phase's occurrences after the files, in declaration order. Each occurrence is bound, its rule's
//! frontier handed over at the rule's first occurrence and its caller's partials taken, all before any evaluates; then each is
//! guarded and evaluated through the file pass's one node step, and handed to its caller in declaration order.
use super::types::{
    HandoverPlace, HostedTables, OccurrenceRun, PhaseBatch, PhaseOccurrence, PhaseState,
    ReadyOccurrence, RuleHandover,
};
use crate::application::file_dag::file_pass::types::PassProgram;
use crate::application::file_dag::frontiers::service::{hand_over, required_files};
use crate::application::file_dag::frontiers::types::HandoverPolicy;
use crate::application::file_dag::program::project_phase::types::{
    HandedFrom, HandoverDeclaration, OccurrenceNumber, PhasePartials, PhaseStop,
};
use crate::application::file_dag::program::rule_outcome::types::{
    ProjectBatch, ProjectOutputOutcome, RuleOutcome,
};
use crate::application::file_dag::project_source::service::read;
use crate::application::file_dag::publications::types::PublicationSlot;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::hosted::service::table_of_query;
use ryo_execution_domain::domain::hosted::types::NumberedFiles;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operating::types::AfterTheFiles;
use ryo_language::domain::symbols::types::{QuerySymbols, Symbols};
use ryo_relations::domain::catalog::service::scope_of;
use ryo_relations::domain::catalog::types::RowScope;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plans::service::reads_of;

pub(crate) mod inputs;
pub(crate) mod occurrence;
pub(crate) mod settle;
pub(crate) mod step;
use occurrence::service::run_occurrence;

/// The project's relations the rules of `rules` read after the files, each once, in the order they are first read: those
/// their frontiers hand over, then those their steps read. What the phase's one read of the project asks for.
pub(crate) fn project_reads(pass: &PassProgram, rules: &[RuleIndex]) -> Vec<Relation> {
    let mut reads = Vec::new();
    for rule in rules {
        let Some(plan) = pass.plans.get(rule.0) else {
            continue;
        };
        let framed = plan.frontier.iter().filter_map(|input| match input {
            Input::Relation(relation) => Some(*relation),
            Input::Step(_) | Input::BoundObservation(_) => None,
        });
        for relation in framed.chain(reads_of(plan).0) {
            if scope_of(relation) == RowScope::Project && !reads.contains(&relation) {
                reads.push(relation);
            }
        }
    }
    reads
}

/// One batch of declared occurrences in one call: each occurrence's rule handed over once `before` admits it, every
/// occurrence's partials taken, then each guarded and evaluated in declaration order, and each outcome handed to `complete` with
/// its slot. A failure of one occurrence stays with it; occurrences after the guard's refusal are absent.
#[expect(
    clippy::too_many_arguments,
    reason = "the phase's four callbacks are the seam's own, taken as the interpreter took them"
)]
pub(crate) fn run_batch<H, T, E>(
    batch: &PhaseBatch<'_, '_>,
    state: &mut PhaseState,
    occurrences: &[PhaseOccurrence],
    symbols: &mut Symbols,
    mut before: impl FnMut(OccurrenceNumber) -> Result<H, E>,
    mut partials: impl FnMut(OccurrenceNumber) -> Result<PhasePartials, E>,
    mut guard: impl FnMut(RuleIndex) -> Result<(), E>,
    mut complete: impl FnMut(
        OccurrenceNumber,
        RuleOutcome<ProjectOutputOutcome>,
        PublicationSlot,
        &mut Symbols,
    ) -> Result<T, E>,
) -> ProjectBatch<Result<T, PhaseStop<E>>, E> {
    let handed = occurrences
        .iter()
        .enumerate()
        .map(|(at, occurrence)| {
            // What `before` returns is held until the occurrence's rule is handed over.
            let _held = before(OccurrenceNumber(at)).map_err(PhaseStop::Caller)?;
            // The project is read at the first occurrence its caller admits: a refused one reads nothing.
            read(batch.project, symbols);
            handover_of(batch, state, occurrence, symbols)
        })
        .collect::<Vec<_>>();
    let ready = handed
        .into_iter()
        .enumerate()
        .map(|(at, handed)| {
            let handover = handed?;
            let partials = partials(OccurrenceNumber(at)).map_err(PhaseStop::Caller)?;
            Ok(ReadyOccurrence { handover, partials })
        })
        .collect::<Vec<Result<ReadyOccurrence, PhaseStop<E>>>>();
    if state.hosted.is_none() {
        state.hosted = Some(hosted_tables(batch, symbols));
    }
    let mut evaluated = Vec::with_capacity(ready.len());
    let mut cohorts: Vec<(RuleIndex, &PhasePartials)> = Vec::new();
    let mut refused_by = None;
    for (occurrence, entry) in occurrences.iter().zip(&ready) {
        let Ok(ReadyOccurrence { handover, partials }) = entry else {
            continue;
        };
        // An occurrence of a rule already evaluated with the same partials in this batch is of its cohort: guarded once.
        let shared = cohorts.iter().any(|(rule, seen)| {
            *rule == occurrence.rule
                && seen.evaluation == partials.evaluation
                && seen.publication == partials.publication
        });
        if !shared {
            if let Err(error) = guard(occurrence.rule) {
                refused_by = Some(error);
                break;
            }
            cohorts.push((occurrence.rule, partials));
        }
        let run = OccurrenceRun {
            rule: occurrence.rule,
            evaluation: &partials.evaluation,
            publication: &partials.publication,
        };
        evaluated.push(run_occurrence(
            batch, state, *handover, occurrence, run, symbols,
        ));
    }
    let mut evaluated = evaluated.into_iter();
    let mut results = Vec::with_capacity(ready.len());
    for ((at, entry), occurrence) in ready.into_iter().enumerate().zip(occurrences) {
        let result = match entry {
            Err(stop) => Err(stop),
            Ok(_) => {
                let Some(outcome) = evaluated.next() else {
                    // Past the guard's refusal no occurrence has an outcome.
                    if refused_by.is_some() {
                        break;
                    }
                    results.push(Err(PhaseStop::Unevaluated));
                    continue;
                };
                outcome.map_err(PhaseStop::Operating).and_then(|outcome| {
                    complete(OccurrenceNumber(at), outcome, occurrence.slot, symbols)
                        .map_err(PhaseStop::Caller)
                })
            }
        };
        results.push(result);
    }
    match refused_by {
        Some(error) => ProjectBatch::Refused {
            completed: results,
            error,
        },
        None => ProjectBatch::Complete(results),
    }
}

/// The occurrence's rule handed over, at its first occurrence for its readiness; later occurrences find it. Answers the place
/// of the rule's handover in the phase.
fn handover_of<E>(
    batch: &PhaseBatch<'_, '_>,
    state: &mut PhaseState,
    occurrence: &PhaseOccurrence,
    symbols: &mut Symbols,
) -> Result<HandoverPlace, PhaseStop<E>> {
    let rule = occurrence.rule;
    let first = *batch
        .first
        .get(rule.0)
        .ok_or(PhaseStop::MissingFirstBucket)?;
    if batch.scoping.inapplicable.get(rule.0).is_none()
        || batch.scoping.refused.get(rule.0).is_none()
    {
        return Err(PhaseStop::Operating(OperatingError::NotRun));
    }
    let required = required_files(batch.program, batch.execution, rule, batch.scoping)
        .map_err(PhaseStop::Operating)?;
    let from = HandedFrom {
        files: required.files.clone(),
        covered: required.covered.clone(),
        skipped: required.skipped.clone(),
        inapplicable: required.inapplicable.clone(),
        refused: required.refused.clone(),
    };
    // A phase hands a rule over from the files it first declared for it: another declaration of them is refused.
    match state.declared.iter().find(|each| each.rule == rule) {
        Some(declared) if declared.from != from => {
            return Err(PhaseStop::Operating(OperatingError::WrongOperation));
        }
        Some(_) => {}
        None => state.declared.push(HandoverDeclaration { rule, from }),
    }
    if let Some(place) = state
        .handovers
        .iter()
        .position(|each| each.rule == rule && each.ready == occurrence.ready)
    {
        return Ok(HandoverPlace(place));
    }
    let policy = match &occurrence.ready {
        Some(ready) => HandoverPolicy::Observations { ready },
        None => HandoverPolicy::Diagnostic,
    };
    let handed = hand_over(
        batch.program,
        rule,
        first,
        batch.facts,
        symbols,
        &required,
        policy,
        batch.project,
        &mut state.withholdings,
    )
    .map_err(PhaseStop::Frontiers)?;
    state.handovers.push(RuleHandover {
        rule,
        ready: occurrence.ready.clone(),
        handed,
    });
    Ok(HandoverPlace(state.handovers.len() - 1))
}

/// The host's rows of each declared relation, interned in the phase's symbols.
fn hosted_tables(batch: &PhaseBatch<'_, '_>, symbols: &mut Symbols) -> HostedTables {
    let paths = batch.execution.services.request().paths();
    HostedTables(
        batch
            .program
            .hosted()
            .iter()
            .map(|relation| {
                table_of_query(
                    relation,
                    NumberedFiles(paths),
                    &mut QuerySymbols::from(&mut *symbols),
                    AfterTheFiles::After,
                )
            })
            .collect(),
    )
}
