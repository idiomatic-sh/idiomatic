//! One occurrence's steps after the files: its emit and named outputs that run after the files, answered through the file
//! pass's one node step with the coverage of what the files and the project handed over, and its steps' fed rows reported.
use crate::application::file_dag::file_pass::types::NodeAnswer;
use crate::application::file_dag::frontiers::service::reason;
use crate::application::file_dag::frontiers::types::FrontierRows;
use crate::application::file_dag::program::facade_runtime::file_execution::service::classify;
use crate::application::file_dag::program::observed_publication::service::ObservedPublication;
use crate::application::file_dag::program::rule_outcome::types::{
    ProjectObservation, ProjectOutputOutcome, RootOutcome, RuleOutcome,
};
use crate::application::file_dag::project_pass::service::step::service::{
    StepRun, ordinal, with_coverage,
};
use crate::application::file_dag::project_pass::types::{
    HandoverPlace, HandoverReads, HostedTables, LastHandover, OccurrenceRun, PhaseBatch,
    PhaseInputs, PhaseOccurrence, PhaseState,
};
use crate::application::file_dag::publications::service::bound::service::{
    bound_coverage, held_observation,
};
use ryo_execution_domain::domain::completion::types::CoverageState;
use ryo_execution_domain::domain::completion::types::completion_policy::value::CompletionPolicy;
use ryo_execution_domain::domain::executable_program::service::rule as rule_nodes;
use ryo_execution_domain::domain::executable_program::service::scopes::service::{
    emit_scope, observation_reach, observation_scope,
};
use ryo_execution_domain::domain::executable_program::types::ObservationReach;
use ryo_execution_domain::domain::frontier_coverage::service::affected_files;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::spelling::service::check_name_spellings;
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::types::Symbols;
use ryo_operators::domain::physical_plan::types::PlanNodeId;
use ryo_operators::domain::physical_plan::types::bound_observation_place::value::BoundObservationPlace;
use ryo_operators::domain::physical_plan::types::held_rows::value::HeldRows;
use ryo_relations::domain::relations::types::{Relation, Value};
use ryo_values::domain::plan_parts::types::{Input, StepScope};
use ryo_values::domain::records::types::ObservationIndex;
use ryo_values::domain::rows::types::Table;
use std::sync::Arc;

/// One occurrence's steps after the files: its emit when it runs after the files, then each of its named outputs that does,
/// then its steps' fed rows reported. The rows of a node that reads a bound observation are released at the occurrence's end.
pub(crate) fn run_occurrence(
    batch: &PhaseBatch<'_, '_>,
    state: &mut PhaseState,
    handover: HandoverPlace,
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
        declared: _,
    } = state;
    let program = &batch.pass.program;
    // Nodes over a rule's frontier answer the handover they ran over: another handover of the rule runs them again.
    match last.iter_mut().find(|each| each.rule == rule) {
        Some(each) if each.place != handover => {
            each.place = handover;
            let mut node = PlanNodeId(0);
            while observation_reach(program, node).is_ok() {
                buffers.release(node);
                node = PlanNodeId(node.0.saturating_add(1));
            }
        }
        Some(_) => {}
        None => last.push(LastHandover {
            rule,
            place: handover,
        }),
    }
    let handed = &handovers
        .get(handover.0)
        .ok_or(OperatingError::NotRun)?
        .handed;
    // Names the files handed over that share a hash are spelled the same, before any step reads them.
    check_name_spellings(
        handed
            .0
            .iter()
            .flat_map(|entry| match entry {
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
        unhanded: std::cell::Cell::new(HandoverReads::Handed),
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
                Ok(NodeAnswer { table, coverage }) => root_of(table, &coverage),
                Err(error) => classify(error),
            },
            Err(error) => RootOutcome::Failed(error),
        },
        phase => RootOutcome::Deferred { phase },
    };
    let metadata = batch.program.rule_metadata(rule)?;
    let mut observations = Vec::new();
    for (index, root) in metadata.observations.iter().enumerate() {
        let output =
            ObservationIndex(u32::try_from(index).map_err(|_| OperatingError::ShapeMismatch)?);
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
                    HeldRows::Held(table) => Ok(NodeAnswer {
                        table: table.clone(),
                        coverage: bound_coverage(&occurrence.bound, id)
                            .cloned()
                            .unwrap_or(CoverageState::Complete),
                    }),
                    HeldRows::Withheld(token) => Err(reason(step.withholdings, token)),
                }
            }
            Some(Input::Relation(_)) | None => Err(OperatingError::NotRun),
        };
        let outcome = match answered {
            Ok(NodeAnswer { table, coverage }) => {
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
                    coverage @ (CoverageState::Partial { .. }
                    | CoverageState::Inapplicable { .. }) => {
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
        CoverageState::Partial { .. } | CoverageState::Inapplicable { .. } => {
            RootOutcome::Partial {
                table,
                affected: affected_files(coverage).unwrap_or_default(),
            }
        }
    }
}
