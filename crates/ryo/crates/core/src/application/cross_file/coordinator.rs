//! Execute declared ready batches and transfer publications before their final consumer retires.
use super::errors::CrossError;
use super::{service::CrossFile, types::ObservationRun};
use crate::application::run_coordination::settings::CrossingInput;
use crate::domain::answering::{
    service::settle,
    types::{Answered, Todo},
};
use ryo_execution::inbound::use_cases::execution::interface::{
    ExecutionRequest, ObservationExecution, ObservationPublication, PreparedExecution,
};
use ryo_execution::inbound::use_cases::execution::types::{
    BoundInput, FrontierDemand, FrontierRequest, ProjectScope, PublicationSlot,
};
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::stored_facts::types::ArenaBytes;

struct PreparedTask {
    todo: Todo,
    slot: PublicationSlot,
    bindings: Vec<BoundInput>,
    scope: ProjectScope,
    frontier_ready: Vec<bool>,
}
impl CrossFile {
    pub(crate) fn observations(
        &self,
        run: ObservationRun<'_>,
        arenas: &mut Vec<ArenaBytes>,
        answered: &mut Answered,
    ) -> Result<(), CrossError> {
        let batches = run.batches;
        let mut stage = run.services.observations(arenas)?;
        let mut selected = vec![false; run.domain.metadata.len()];
        for todo in run.todo {
            *selected
                .get_mut(todo.rule.0)
                .ok_or(OperatingError::NotRun)? = true;
        }
        for (rule, selected) in selected.iter().enumerate() {
            if !selected {
                stage.finish(RuleIndex(rule))?;
            }
        }
        let readiness = run
            .domain
            .observations
            .iter()
            .map(|frame| {
                ryo_execution::inbound::use_cases::execution::types::FileObservationReadiness {
                    file: frame.file,
                    root: std::sync::Arc::clone(&frame.root),
                }
            })
            .collect::<Vec<_>>();
        stage.restore_frontiers(
            run.facts,
            run.domain.files,
            run.domain.covered,
            run.domain.skipped,
            &readiness,
            run.first_bucket,
            &selected,
        )?;
        // The retained project execution and every decoded text place share one
        // live symbol generation across all producer and consumer batches.
        let mut symbols = Symbols::default();
        for batch in batches {
            let mut tasks = Vec::new();
            for rule in batch {
                let Some(todo) = run.todo.iter().find(|todo| todo.rule == *rule) else {
                    continue;
                };
                let readiness = stage.binding_readiness(*rule)?;
                let dependencies = run.program.frontier_dependencies(*rule)?;
                let frontier_ready = dependencies
                    .iter()
                    .map(|inputs| {
                        inputs
                            .iter()
                            .all(|input| readiness.get(input).copied() == Some(true))
                    })
                    .collect();
                tasks.push(PreparedTask {
                    todo: todo.clone(),
                    slot: stage.reserve(*rule)?,
                    bindings: stage.bindings(*rule)?,
                    scope: stage.project_scope(
                        *rule,
                        run.domain.files,
                        run.domain.covered,
                        run.domain.skipped,
                    )?,
                    frontier_ready,
                });
            }
            if tasks.is_empty() {
                continue;
            }
            let results = {
                let crossing = CrossingInput {
                    arenas: stage.arenas(),
                    services: run.services,
                    program: std::sync::Arc::clone(&run.program),
                    domain: run.domain.clone(),
                };
                let mut prepared = Vec::with_capacity(tasks.len());
                let mut errors = Vec::with_capacity(tasks.len());
                for task in &tasks {
                    match prepare_frontiers(
                        &run,
                        &task.todo,
                        &task.scope,
                        &task.frontier_ready,
                        &mut symbols,
                    ) {
                        Ok(frontier) => {
                            errors.push(None);
                            prepared.push(super::types::ProjectTask {
                                todo: &task.todo,
                                frontier,
                                bound: &task.bindings,
                                slot: task.slot,
                                scope: &task.scope,
                            });
                        }
                        Err(error) => errors.push(Some(error)),
                    }
                }
                let mut evaluated = self
                    .evaluate_batch(&crossing, prepared, &stage, &mut symbols)
                    .into_iter();
                errors
                    .into_iter()
                    .zip(&tasks)
                    .map(|(error, task)| match error {
                        Some(error) => Err(error),
                        None => evaluated.next().unwrap_or_else(|| {
                            Err(crate::domain::rules::service::failed(
                                &task.todo.id,
                                &OperatingError::NotRun,
                            ))
                        }),
                    })
                    .collect::<Vec<_>>()
            };
            let mut first_error = None;
            for (mut task, result) in tasks.into_iter().zip(results) {
                task.bindings.clear();
                match result {
                    Ok(result) => {
                        if let Some(transfer) = settle(
                            &task.todo,
                            result.crossed,
                            &mut answered.later,
                            &mut answered.fed,
                            &mut answered.unknowns,
                        ) {
                            if let Err(error) = stage.install(
                                task.todo.rule,
                                task.slot,
                                transfer,
                                result.publications,
                            ) {
                                first_error.get_or_insert(CrossError::from(error));
                            }
                        }
                    }
                    Err(error) => {
                        first_error.get_or_insert(CrossError::from(error));
                    }
                }
                if let Err(error) = stage.finish(task.todo.rule) {
                    first_error.get_or_insert(CrossError::from(error));
                }
            }
            if let Some(error) = first_error {
                return Err(error);
            }
        }
        stage.finish_stage()?;
        Ok(())
    }
}
fn prepare_frontiers(
    run: &ObservationRun<'_>,
    todo: &Todo,
    scope: &ProjectScope,
    ready: &[bool],
    symbols: &mut Symbols,
) -> Result<
    ryo_execution::inbound::use_cases::execution::types::PreparedFrontiers,
    ryo_values::domain::rule_requests::errors::RulesError,
> {
    let failed =
        |error: &dyn std::fmt::Display| crate::domain::rules::service::failed(&todo.id, error);
    let first = *run
        .first_bucket
        .get(todo.rule.0)
        .ok_or_else(|| failed(&OperatingError::NotRun))?;
    run.services
        .prepare_frontiers(
            FrontierRequest {
                scope,
                rule: todo.rule,
                first,
                facts: run.facts,
                project: run.project,
                project_symbols: run.project_symbols,
                demand: FrontierDemand::Observations { ready },
            },
            symbols,
        )
        .map_err(|error| failed(&error))
}

use ryo_execution::inbound::use_cases::execution::types::{OperatingError, RuleIndex};
