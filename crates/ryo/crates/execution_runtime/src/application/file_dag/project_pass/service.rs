//! The `project_pass` service: a phase's occurrences after the files, in declaration order. Each occurrence is bound, its rule's
//! frontier handed over at the rule's first occurrence and its caller's partials taken, all before any evaluates; then each is
//! guarded and evaluated through the file pass's one node step, and handed to its caller in declaration order.

use super::types::{HostedTables, PhaseBatch, OccurrenceRun, PhaseInputs, PhaseOccurrence, PhaseState, RuleHandover};
use crate::application::file_dag::file_pass::types::PassProgram;
use crate::application::file_dag::frontiers::service::{hand_over, reason, required_files};
use crate::application::file_dag::frontiers::types::{FrontierRows, HandoverPolicy};
use crate::application::file_dag::program::facade_runtime::file_execution::service::classify;
use crate::application::file_dag::program::observed_publication::service::ObservedPublication;
use crate::application::file_dag::program::project_phase::types::{
    OccurrenceNumber, PhasePartials, PhaseStop,
};
use crate::application::file_dag::program::rule_outcome::types::{
    ProjectBatch, ProjectObservation, ProjectOutputOutcome, RootOutcome, RuleOutcome,
};
use crate::application::file_dag::project_pass::service::step::service::{StepRun, ordinal, with_coverage};
use crate::application::file_dag::publications::service::{bound_coverage, held_observation};
use crate::application::file_dag::publications::types::PublicationSlot;
use ryo_execution_domain::domain::completion::types::CoverageState;
use ryo_execution_domain::domain::completion::types::completion_policy::value::CompletionPolicy;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::executable_program::service::{
    emit_scope, observation_reach, observation_scope, rule as rule_nodes, serving,
};
use ryo_execution_domain::domain::executable_program::types::{ObservationReach, Serving};
use ryo_execution_domain::domain::frontier_coverage::service::affected_files;
use ryo_execution_domain::domain::hosted::service::table_of_query;
use ryo_execution_domain::domain::hosted::types::NumberedFiles;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operating::types::AfterTheFiles;
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::types::{QuerySymbols, Symbols};
use ryo_operators::domain::physical_plan::types::PlanNodeId;
use ryo_operators::domain::physical_plan::types::bound_observation_place::value::BoundObservationPlace;
use ryo_operators::domain::physical_plan::types::held_rows::value::HeldRows;
use ryo_relations::domain::catalog::service::scope_of;
use ryo_relations::domain::catalog::types::RowScope;
use ryo_execution_domain::domain::spelling::service::check_name_spellings;
use ryo_relations::domain::relations::types::{Relation, Value};
use ryo_values::domain::plan_parts::types::{Input, StepScope};
use ryo_values::domain::plans::service::{inputs_of, named};
use ryo_values::domain::records::types::ObservationIndex;
use ryo_values::domain::rows::types::Table;
use std::sync::Arc;

pub(crate) mod inputs;
pub(crate) mod step;


/// The project's relations the steps of `rules` read, each once, in the order they are first read: what the phase's one read
/// of the project asks for.
pub(crate) fn project_reads(pass: &PassProgram, rules: &[RuleIndex]) -> Vec<Relation> {
    let mut reads = Vec::new();
    for rule in rules {
        let Some(plan) = pass.plans.get(rule.0) else {
            continue;
        };
        for step in &plan.steps {
            for input in named(&inputs_of(&step.operation)) {
                if let Input::Relation(relation) = *input
                    && scope_of(relation) == RowScope::Project
                    && !reads.contains(&relation)
                {
                    reads.push(relation);
                }
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
            handover_of(batch, state, occurrence, symbols)
        })
        .collect::<Vec<_>>();
    let ready = handed
        .into_iter()
        .enumerate()
        .map(|(at, handed)| {
            let place = handed?;
            let partials = partials(OccurrenceNumber(at)).map_err(PhaseStop::Caller)?;
            Ok((place, partials))
        })
        .collect::<Vec<Result<(usize, PhasePartials), PhaseStop<E>>>>();
    if state.hosted.is_none() {
        state.hosted = Some(hosted_tables(batch, symbols));
    }
    let mut evaluated = Vec::with_capacity(ready.len());
    let mut cohorts: Vec<(RuleIndex, &PhasePartials)> = Vec::new();
    let mut refused_by = None;
    for (occurrence, entry) in occurrences.iter().zip(&ready) {
        let Ok((place, partials)) = entry else {
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
        evaluated.push(run_occurrence(batch, state, *place, occurrence, run, symbols));
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
) -> Result<usize, PhaseStop<E>> {
    let rule = occurrence.rule;
    if let Ok(Serving::NotServedUntil(stage)) = serving(&batch.pass.program, rule) {
        return Err(PhaseStop::Operating(OperatingError::NotServedUntil(stage)));
    }
    let first = *batch.first.get(rule.0).ok_or(PhaseStop::MissingFirstBucket)?;
    if batch.scoping.inapplicable.get(rule.0).is_none() || batch.scoping.refused.get(rule.0).is_none() {
        return Err(PhaseStop::Operating(OperatingError::NotRun));
    }
    let required = required_files(batch.program, batch.request, rule, batch.scoping)
        .map_err(PhaseStop::Operating)?;
    if let Some(place) = state
        .handovers
        .iter()
        .position(|each| each.rule == rule && each.ready == occurrence.ready)
    {
        return Ok(place);
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
    Ok(state.handovers.len() - 1)
}

/// The host's rows of each declared relation, interned in the phase's symbols.
fn hosted_tables(batch: &PhaseBatch<'_, '_>, symbols: &mut Symbols) -> HostedTables {
    let paths = batch.request.paths();
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

/// One occurrence's steps after the files: its emit when it runs after the files, then each of its named outputs that does,
/// then its steps' fed rows reported. The rows of a node that reads a bound observation are released at the occurrence's end.
fn run_occurrence(
    batch: &PhaseBatch<'_, '_>,
    state: &mut PhaseState,
    handover: usize,
    occurrence: &PhaseOccurrence,
    run: OccurrenceRun<'_>,
    symbols: &mut Symbols,
) -> Result<RuleOutcome<ProjectOutputOutcome>, OperatingError> {
    let rule = run.rule;
    let PhaseState {
        buffers,
        handovers,
        withholdings,
        hosted,
        last,
    } = state;
    let program = &batch.pass.program;
    // Nodes over a rule's frontier answer the handover they ran over: another handover of the rule runs them again.
    match last.iter_mut().find(|(each, _)| *each == rule) {
        Some((_, place)) if *place != handover => {
            *place = handover;
            let mut node = PlanNodeId(0);
            while observation_reach(program, node).is_ok() {
                buffers.release(node);
                node = PlanNodeId(node.0.saturating_add(1));
            }
        }
        Some(_) => {}
        None => last.push((rule, handover)),
    }
    let handed = &handovers.get(handover).ok_or(OperatingError::NotRun)?.handed;
    // Names the files handed over that share a hash are spelled the same, before any step reads them.
    check_name_spellings(
        handed.0.iter().flat_map(|entry| match entry {
            FrontierRows::Held { table, .. } => table.cells.as_slice(),
            FrontierRows::Withheld(_) | FrontierRows::Elsewhere => &[],
        })
        .filter_map(|cell| match cell {
            Value::Name(name) => Some(*name),
            _ => None,
        }),
        symbols,
        batch.arenas,
    )?;
    let empty = HostedTables::default();
    let plan = Arc::clone(batch.pass.plans.get(rule.0).ok_or(OperatingError::NotRun)?);
    let inputs = PhaseInputs {
        handed,
        bound: &occurrence.bound,
        hosted: hosted.as_ref().unwrap_or(&empty),
        frontier: &plan.frontier,
        unhanded: std::cell::Cell::new(false),
    };
    let completion = CompletionPolicy::project_inputs(rule, run.evaluation, run.publication);
    let nodes = rule_nodes(program, rule)?.clone();
    let elements = Elements::default();
    let mut step = StepRun {
        batch,
        buffers,
        withholdings,
        inputs: &inputs,
        elements: &elements,
        rule,
        completion: &completion,
    };
    let emit = match emit_scope(program, rule)? {
        StepScope::CrossFile => match nodes.emit.clone() {
            Ok(node) => match step.answer(symbols, node, plan.emit.step) {
                Ok((table, coverage)) => root_of(table, &coverage),
                Err(error) => classify(error),
            },
            Err(error) => RootOutcome::Failed(error),
        },
        phase => RootOutcome::Deferred { phase },
    };
    let metadata = batch.program.rule_metadata(rule)?;
    let mut observations = Vec::new();
    for (index, root) in metadata.observations.iter().enumerate() {
        let output = ObservationIndex(u32::try_from(index).map_err(|_| OperatingError::ShapeMismatch)?);
        if observation_scope(program, rule, output)? != StepScope::CrossFile {
            continue;
        }
        let answered = match plan.observations.get(index).map(|each| each.source) {
            Some(Input::Step(id)) => nodes
                .steps
                .get(usize::try_from(id.0).unwrap_or(usize::MAX))
                .ok_or(OperatingError::NotRun)
                .and_then(Clone::clone)
                .and_then(|node| step.answer(symbols, node, id)),
            Some(Input::BoundObservation(id)) => {
                let place = BoundObservationPlace::of(ordinal(rule)?, id);
                match held_observation(&occurrence.bound, place) {
                    HeldRows::Held(table) => Ok((
                        table.clone(),
                        bound_coverage(&occurrence.bound, id).cloned().unwrap_or(CoverageState::Complete),
                    )),
                    HeldRows::Withheld(token) => Err(reason(step.withholdings, token)),
                }
            }
            Some(Input::Relation(_)) | None => Err(OperatingError::NotRun),
        };
        let outcome = match answered {
            Ok((table, coverage)) => {
                match with_coverage(coverage, completion.coverage(&root.reads)) {
                    CoverageState::Complete => {
                        let rows = Arc::new(table.clone());
                        ProjectObservation::Complete {
                            table,
                            publication: ObservedPublication {
                                rows,
                                coverage: CoverageState::Complete,
                                stage: Arc::downgrade(batch.stage),
                            },
                        }
                    }
                    coverage @ (CoverageState::Partial { .. } | CoverageState::Inapplicable { .. }) => {
                        let rows = Arc::new(table.clone());
                        ProjectObservation::Partial {
                            table,
                            publication: ObservedPublication {
                                rows,
                                coverage,
                                stage: Arc::downgrade(batch.stage),
                            },
                        }
                    }
                }
            }
            Err(error) => ProjectObservation::Other(classify(error)),
        };
        observations.push(ProjectOutputOutcome { output, outcome });
    }
    if let Some(observer) = batch.observer {
        let fed = step.fed(symbols, &plan, &nodes)?;
        observer.fed(rule, fed)?;
    }
    // A node that reads a bound observation answers this occurrence's slots only.
    let mut node = PlanNodeId(0);
    while let Ok(reach) = observation_reach(program, node) {
        if reach == ObservationReach::Occurrence {
            step.buffers.release(node);
        }
        node = PlanNodeId(node.0.saturating_add(1));
    }
    Ok(RuleOutcome { emit, observations })
}

/// A root's answer from its rows and their coverage.
fn root_of(table: Table, coverage: &CoverageState) -> RootOutcome {
    match coverage {
        CoverageState::Complete => RootOutcome::Complete(table),
        CoverageState::Partial { .. } | CoverageState::Inapplicable { .. } => RootOutcome::Partial {
            table,
            affected: affected_files(coverage).unwrap_or_default(),
        },
    }
}

