//! The `project_pass` service: a phase's occurrences after the files, in declaration order. Each occurrence is bound, its rule's
//! frontier read from the store and proven at the rule's first occurrence and its caller's partials taken, all before any
//! evaluates; then each is guarded, its rule's frontier decoded at the rule's first occurrence that evaluates, and evaluated
//! through the file pass's one node step; after the rule's last occurrence its handover and rows go. Each outcome is handed to
//! its caller in declaration order.
use super::types::{
    HandoverPlace, HandoverState, HostedTables, OccurrenceRun, OccurrencesLeft, PhaseBatch,
    PhaseOccurrence, PhaseState, ReadyOccurrence, RuleHandover, RuleUse, RuleUses,
};
use crate::application::file_dag::file_pass::types::PassProgram;
use crate::application::file_dag::frontiers::service::{decode, policy_of, prove, required_files};
use crate::application::file_dag::program::project_phase::types::{
    HandedFrom, HandoverDeclaration, OccurrenceNumber, PhasePartials, PhaseStop,
};
use crate::application::file_dag::program::rule_outcome::types::{
    ProjectBatch, ProjectOutputOutcome, RuleOutcome,
};
use crate::application::file_dag::project_source::service::read;
use crate::application::file_dag::publications::types::PublicationSlot;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::executable_program::service::rule as rule_nodes;
use ryo_execution_domain::domain::hosted::service::table_of_query;
use ryo_execution_domain::domain::hosted::types::NumberedFiles;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operating::types::AfterTheFiles;
use ryo_language::domain::symbols::types::{QuerySymbols, Symbols};
use ryo_relations::domain::catalog::service::scope_of;
use ryo_relations::domain::catalog::types::RowScope;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::query_plans::service::reads_of;

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

/// How many times each rule occurs among `rules`, the declared occurrences of a phase.
pub(crate) fn uses_of(rules: impl IntoIterator<Item = RuleIndex>) -> RuleUses {
    let mut uses = RuleUses::default();
    for rule in rules {
        match uses.0.iter_mut().find(|each| each.rule == rule) {
            Some(each) => each.left = OccurrencesLeft(each.left.0.saturating_add(1)),
            None => uses.0.push(RuleUse {
                rule,
                left: OccurrencesLeft(1),
            }),
        }
    }
    uses
}

/// One occurrence of `rule` was evaluated: one fewer is still to come. After its last, the rule's handovers and the rows of its
/// emit and steps go, and its nodes no longer stand over a handover.
fn used(batch: &PhaseBatch<'_, '_>, state: &mut PhaseState, rule: RuleIndex) {
    let Some(each) = state.uses.0.iter_mut().find(|each| each.rule == rule) else {
        return;
    };
    each.left = OccurrencesLeft(each.left.0.saturating_sub(1));
    if each.left.0 > 0 {
        return;
    }
    for handover in state.handovers.iter_mut().filter(|each| each.rule == rule) {
        handover.handed = HandoverState::Released;
    }
    if let Ok(nodes) = rule_nodes(&batch.pass.program, rule) {
        for node in nodes
            .steps
            .iter()
            .chain(std::iter::once(&nodes.emit))
            .flatten()
        {
            state.buffers.release(*node);
        }
    }
    state.last.retain(|each| each.rule != rule);
}

/// One batch of declared occurrences in one call: each occurrence's rule read from the store and proven once `before` admits
/// it, every occurrence's partials taken, then each guarded, its rule decoded at the first that evaluates, evaluated in
/// declaration order, its rule's handover and rows released after the rule's last, and each outcome handed to `complete` with
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
            // What `before` returns is held until the occurrence's rule is read from the store and proven.
            let _held = before(OccurrenceNumber(at)).map_err(PhaseStop::Caller)?;
            // The project is read at the first occurrence its caller admits: a refused one reads nothing.
            read(batch.project, symbols);
            handover_of(batch, state, occurrence)
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
        // The rule's handover is decoded at its first occurrence that evaluates, and goes after its last.
        evaluated.push(
            handed_over(batch, state, *handover, symbols).and_then(|()| {
                run_occurrence(batch, state, *handover, occurrence, run, symbols)
                    .map_err(PhaseStop::Operating)
            }),
        );
        used(batch, state, occurrence.rule);
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
                outcome.and_then(|outcome| {
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

/// The occurrence's rule read from the store and proven, at its first occurrence for its readiness; later occurrences find it.
/// Answers the place of the rule's handover in the phase.
fn handover_of<E>(
    batch: &PhaseBatch<'_, '_>,
    state: &mut PhaseState,
    occurrence: &PhaseOccurrence,
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
    if let Some(place) = state.handovers.iter().position(|each| {
        each.rule == rule
            && each.ready == occurrence.ready
            && !matches!(each.handed, HandoverState::Released)
    }) {
        return Ok(HandoverPlace(place));
    }
    let policy = policy_of(occurrence.ready.as_ref());
    let proven = prove(
        batch.program,
        rule,
        first,
        batch.facts,
        &required,
        policy,
        batch.project,
    )
    .map_err(PhaseStop::Frontiers)?;
    state.handovers.push(RuleHandover {
        rule,
        ready: occurrence.ready.clone(),
        handed: HandoverState::Proven { proven, required },
    });
    Ok(HandoverPlace(state.handovers.len() - 1))
}

/// The handover at `place` decoded into the phase's symbols, when it is only proven: at the first occurrence of its rule that
/// evaluates.
fn handed_over<E>(
    batch: &PhaseBatch<'_, '_>,
    state: &mut PhaseState,
    place: HandoverPlace,
    symbols: &mut Symbols,
) -> Result<(), PhaseStop<E>> {
    let PhaseState {
        handovers,
        withholdings,
        ..
    } = state;
    let handover = handovers
        .get_mut(place.0)
        .ok_or(PhaseStop::Operating(OperatingError::NotRun))?;
    let HandoverState::Proven { proven, required } = &handover.handed else {
        return Ok(());
    };
    let first = *batch
        .first
        .get(handover.rule.0)
        .ok_or(PhaseStop::MissingFirstBucket)?;
    let policy = policy_of(handover.ready.as_ref());
    let handed = decode(
        batch.program,
        handover.rule,
        first,
        batch.facts,
        symbols,
        required,
        policy,
        proven,
        withholdings,
    )
    .map_err(PhaseStop::Frontiers)?;
    handover.handed = HandoverState::Handed(handed);
    Ok(())
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
