//! Plan result delivery uses the mandatory application-owned operator engine.
use super::executor::PrimitiveExecutor;
use crate::domain::coverage::types::Partials;
use crate::domain::operating::{errors::OperatingError, types::OperatorContext};
use crate::domain::operator::contract::types::OperatorContract;
use crate::domain::operator::types::RowsFed;
use crate::domain::table_model::{
    service::{count_of, kinds_of},
    types::{Frontier, Rows},
};
use crate::domain::trust::service::files_for;
use ryo_values::domain::plan_parts::{service::step_in, types::Input};
use ryo_values::domain::plans::{
    service::{inputs_of, named, step_scope},
    types::{Operation, Plan, StepScope},
};

/// Whatever an operator refuses, or `NotRun`/`NotRead` when a step reads what nothing provides.
pub(crate) fn evaluate_cross_roots_with_executor(
    completion: &super::completion::CompletionPolicy,
    scope: &super::scope::ExecutionScope<'_>,
    rule: usize,
    plan: &Plan,
    frontier: Vec<crate::application::cross_file::types::FrontierInput>,
    refused: &[crate::domain::crossing::types::RefusedFrontier],
    observation_partials: &Partials,
    context: &mut OperatorContext<'_>,
    fed: &mut [RowsFed],
    hosted: Vec<Table>,
    executor: &super::executor::ScopedExecutor<'_>,
) -> Result<crate::domain::crossing::types::PhaseRoots, OperatingError> {
    use super::interface::DagExecution;
    use crate::domain::file_program::types::NodeId;
    use std::cell::RefCell;
    use std::sync::Arc;
    // This adapter installs phase sources and declares roots. It owns no step
    // schedule, computed-table array, or independent last-consumer algorithm.
    let graph = Arc::clone(scope.graph());
    let schedule = super::activation::types::ActivationCache::default().activate(
        &graph,
        |index| index == rule,
        &[],
    );
    let mut state = super::types::FileDagExecution::new_for_schedule(
        &schedule,
        std::rc::Rc::clone(scope.owner()),
        completion.clone(),
    );
    state.phase = StepScope::CrossFile;
    state.counting = crate::domain::lowering::types::Counting::Counts;
    let resolve = |step: usize| -> Result<NodeId, OperatingError> {
        graph
            .program_nodes
            .get(rule)
            .and_then(|nodes| nodes.get(step))
            .cloned()
            .unwrap_or(Err(OperatingError::NotRun))
    };
    // A handed-over failure is an owned input outcome, not a signal to run a
    // file producer again using project bindings.
    for fault in refused {
        if let Input::Step(step) = fault.input {
            state.provided.insert(
                resolve(step.0 as usize)?,
                super::types::ProvidedInput::Refused {
                    input: fault.input,
                    error: fault.error.clone(),
                },
            );
        }
    }
    let mut relation_tables = Vec::new();
    for input in frontier {
        let each = match input {
            crate::application::cross_file::types::FrontierInput::Compact {
                input: Input::Step(step),
                table,
                ..
            } => {
                state.provided.insert(
                    resolve(step.0 as usize)?,
                    super::types::ProvidedInput::Compact {
                        input: Input::Step(step),
                        table: Arc::new(table),
                    },
                );
                continue;
            }
            crate::application::cross_file::types::FrontierInput::Compact { .. } => {
                return Err(OperatingError::WrongOperation);
            }
            crate::application::cross_file::types::FrontierInput::Decoded(each) => each,
        };
        match each.input {
            Input::Relation(relation) => relation_tables.push((relation, each.table)),
            Input::Step(step) => {
                let publication = scope.owner().computed(each.table)?;
                state.provided.insert(
                    resolve(step.0 as usize)?,
                    super::types::ProvidedInput::Live {
                        input: Input::Step(step),
                        publication,
                    },
                );
            }
            Input::BoundObservation(binding) => {
                let observations = graph.observations.as_ref().ok_or(OperatingError::NotRun)?;
                let at = observations
                    .slots
                    .iter()
                    .position(|slot| slot.rule == rule && slot.input == binding)
                    .ok_or(OperatingError::NotRun)?;
                let id = crate::domain::file_program::types::ObservationGraphInputId(at);
                let publication = scope.owner().computed(each.table)?;
                state.provided_observations.insert(id, Ok(publication));
            }
        }
    }
    // Supplied file producers are leaves in this phase. Only actual emitted or
    // observed roots own root edges; old file-frontier pins cannot retain them.
    let mut roots = Vec::new();
    for output in &plan.observations {
        if output.scope == StepScope::CrossFile {
            let Input::Step(step) = output.source else {
                return Err(OperatingError::NotRun);
            };
            roots.push(resolve(step.0 as usize)?);
        }
    }
    if plan
        .steps
        .get(plan.emit.step.0 as usize)
        .is_some_and(|step| step.scope == StepScope::CrossFile)
    {
        roots.push(resolve(plan.emit.step.0 as usize)?);
    }
    state.remaining = super::activation::service::remaining_for(&graph, roots, |node| {
        state.provided.contains_key(&node)
    });
    state.retire_unused_sources();
    let request_sources = hosted
        .into_iter()
        .map(|table| scope.owner().computed(table))
        .collect::<Result<Vec<_>, _>>()?;
    for (index, step) in plan.steps.iter().enumerate() {
        if let Operation::Hosted { relation } = &step.operation {
            let publication = request_sources
                .get(relation.0 as usize)
                .ok_or(OperatingError::NotRun)?
                .clone();
            state.hosted.insert(resolve(index)?, publication);
        }
    }
    let read = |relation| -> Result<Rows<'_>, OperatingError> {
        if let Some(fault) = refused
            .iter()
            .find(|fault| fault.input == Input::Relation(relation))
        {
            return Err(fault.error.clone());
        }
        relation_tables
            .iter()
            .find(|(source, _)| *source == relation)
            .map(|(_, table)| Rows::Computed(table))
            .ok_or_else(
                || match completion.missing_frontier(Input::Relation(relation)) {
                    super::completion::NodeCompletion::Unavailable(unavailable) => {
                        match unavailable.reason {
                            super::completion::UnavailableReason::MissingFrontier { input } => {
                                OperatingError::FrontierUnavailable { input }
                            }
                        }
                    }
                    _ => OperatingError::WrongOperation,
                },
            )
    };
    let state = RefCell::new(state);
    state.begin_rule(rule)?;
    let publication_reads = crate::domain::trust::service::physical_reads(plan);
    let mut outputs = Vec::new();
    let mut emit = None;
    let mut emit_error = None;
    let mut demand = |step: usize| -> Result<Table, OperatingError> {
        let id = resolve(step)?;
        let result = (|| {
            let publication = state.demand(id, &read, executor, context)?;
            let rows = scope.owner().resolve(publication.as_ref(), &read)?;
            let mut table = Table::default();
            crate::domain::table_model::service::materialize(
                &rows,
                &mut table,
                &mut context.scratch.row,
            );
            Ok(table)
        })();
        // Root delivery consumed this logical reader, even for a deferred error.
        state.borrow_mut().consumed(id);
        state.borrow_mut().observe_provided("provided_after_root");
        result
    };
    // Root order is orchestration only: the common owner schedules all actual
    // dependencies and preserves each independent root's original failure.
    for (index, output) in plan.observations.iter().enumerate() {
        if output.scope != StepScope::CrossFile {
            continue;
        }
        let Input::Step(step) = output.source else {
            return Err(OperatingError::NotRun);
        };
        let result = demand(step.0 as usize);
        let uncertain = files_for(
            observation_partials,
            publication_reads
                .get(step.0 as usize)
                .ok_or(OperatingError::NotRun)?,
        );
        let readiness = match &result {
            Ok(_) if uncertain.is_empty() => {
                ryo_values::domain::observations::types::ProducerReadiness::Complete
            }
            _ => ryo_values::domain::observations::types::ProducerReadiness::Unknown(
                ryo_values::domain::observations::types::ObservationReason(
                    result.as_ref().err().map_or_else(
                        || "producer dependencies are incomplete".into(),
                        |error| error.to_string().into(),
                    ),
                ),
            ),
        };
        let table = if uncertain.is_empty() {
            result.as_ref().ok().cloned()
        } else {
            None
        };
        if step == plan.emit.step {
            match result {
                Ok(table) => emit = Some(table),
                Err(error) => emit_error = Some(error),
            }
        }
        outputs.push(crate::domain::crossing::types::ObservedRoot {
            output: ryo_values::domain::observations::types::ObservationIndex(
                u32::try_from(index).map_err(|_| OperatingError::NotRun)?,
            ),
            table,
            readiness,
        });
    }
    if step_scope(plan, plan.emit.step) == Some(StepScope::CrossFile)
        && emit.is_none()
        && emit_error.is_none()
    {
        match demand(plan.emit.step.0 as usize) {
            Ok(table) => emit = Some(table),
            Err(error) => emit_error = Some(error),
        }
    }
    for (target, counted) in fed.iter_mut().zip(state.counts(rule)?) {
        *target = counted;
    }
    state.borrow_mut().finish_rule(rule);
    Ok(crate::domain::crossing::types::PhaseRoots {
        emit,
        emit_error,
        observations: outputs,
    })
}

/// Legacy diagnostic-only caller of the same execution pass. A per-file Emit is not a cross result.
#[cfg(test)]
pub(crate) fn evaluate_cross_with_executor(
    plan: &Plan,
    frontier: Vec<Frontier>,
    partials: &Partials,
    context: &mut OperatorContext<'_>,
    fed: &mut [RowsFed],
    hosted: &[Table],
    executor: &super::executor::ScopedExecutor<'_>,
    operators: &[std::sync::Arc<OperatorContract>],
) -> Result<Table, OperatingError> {
    let roots = evaluate_cross_roots_with_executor(
        plan,
        frontier,
        &[],
        partials,
        partials,
        context,
        fed,
        hosted,
        executor,
        operators,
    )?;
    if let Some(error) = roots.emit_error {
        return Err(error);
    }
    roots.emit.ok_or(OperatingError::NotRun)
}

#[cfg(test)]
pub(crate) fn compute(
    operation: &Operation,
    inputs: &[Rows<'_>],
    context: &mut OperatorContext<'_>,
    out: &mut Table,
) -> Result<(), OperatingError> {
    let schemas = inputs
        .iter()
        .map(|rows| crate::domain::operation_schema::types::InputSchema {
            kinds: kinds_of(rows),
            tree: match rows {
                Rows::Tree(tree) => Some(tree.form),
                _ => None,
            },
        })
        .collect::<Vec<_>>();
    let contract = super::derived::prepare::operator(
        crate::domain::operator::contract::registration::registered(
            crate::domain::features::service::kind_of(operation),
        ),
        operation,
        &schemas,
        context.bindings,
    )?;
    crate::application::file_dag::execution::CommonOperatorEngine::fixture()
        .collect(&contract, inputs, context, out)
}

#[cfg(test)]
pub(crate) fn evaluate_cross(
    plan: &Plan,
    frontier: Vec<Frontier>,
    partials: &Partials,
    context: &mut OperatorContext<'_>,
    fed: &mut [RowsFed],
    hosted: &[Table],
) -> Result<Table, OperatingError> {
    let bindings = vec![context.bindings.to_vec()];
    let request = crate::domain::operator::contract::types::RequestOperators::prepare(
        std::slice::from_ref(plan),
        &bindings,
        &[],
        crate::outbound::derived::factory::test_port(),
    )?;
    evaluate_cross_with_executor(
        plan,
        frontier,
        partials,
        context,
        fed,
        hosted,
        &crate::application::file_dag::execution::CommonOperatorEngine::fixture(),
        &request.rules[0],
    )
}

use ryo_values::domain::rows::types::Table;
