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

impl CrossFile {
    pub(crate) fn observations(
        &self,
        run: ObservationRun<'_>,
        arenas: &mut Vec<ArenaBytes>,
        answered: &mut Answered,
        mut guard: impl FnMut(RuleIndex) -> Result<(), CrossError>,
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
        let inapplicable = crate::domain::crossing::service::inapplicable_files(&run.domain);
        stage.restore_frontiers(
            run.facts,
            run.domain.files,
            run.domain.covered,
            run.domain.skipped,
            &inapplicable,
            &readiness,
            run.first_bucket,
            &selected,
        )?;
        // The retained project execution and every decoded text place share one
        // live symbol generation across all producer and consumer batches.
        let mut symbols = Symbols::default();
        for batch in batches {
            use super::preparation::{service::Preparation, types::PreparedProject};
            let inventory = Preparation::named(batch, run.todo);
            if inventory.declaration_count().0 == 0 {
                continue;
            }
            let prepared = inventory.prepare_all(|_, todo| {
                let readiness = stage.binding_readiness(todo.rule)?;
                let dependencies = run.program.frontier_dependencies(todo.rule)?;
                let frontier_ready = dependencies
                    .iter()
                    .map(|inputs| {
                        inputs
                            .iter()
                            .all(|input| readiness.get(input).copied() == Some(true))
                    })
                    .collect::<Vec<_>>();
                let slot = stage.reserve(todo.rule)?;
                let bound = stage.bindings(todo.rule)?;
                let scope = stage.project_scope(
                    todo.rule,
                    run.domain.files,
                    run.domain.covered,
                    run.domain.skipped,
                    inapplicable
                        .get(todo.rule.0)
                        .ok_or(OperatingError::NotRun)?,
                )?;
                let frontier =
                    prepare_frontiers(&run, todo, &scope, &frontier_ready, &mut symbols)?;
                Ok(PreparedProject {
                    scope,
                    frontier,
                    bound,
                    slot,
                })
            });
            let sealed = prepared.seal()?;
            let retiring = sealed.declarations();
            let (results, refused) = {
                let crossing = CrossingInput {
                    arenas: stage.arenas(),
                    program: std::sync::Arc::clone(&run.program),
                    domain: run.domain.clone(),
                };
                use ryo_execution::inbound::use_cases::execution::types::ProjectBatch;
                match self.evaluate_batch(&crossing, sealed, &stage, &mut symbols, &mut guard) {
                    ProjectBatch::Complete(completed) => (completed, None),
                    ProjectBatch::Refused { completed, error } => (completed, Some(error)),
                }
            };
            let mut first_error = None;
            let mut outcomes = results.into_iter();
            for todo in retiring.iter() {
                if let Some(outcome) = outcomes.next() {
                    match outcome.result {
                        Ok(result) => {
                            if let Some(transfer) = settle(
                                &todo,
                                result.crossed,
                                &mut answered.later,
                                &mut answered.fed,
                                &mut answered.unknowns,
                            ) {
                                if let Err(error) = stage.install(
                                    todo.rule,
                                    result.slot,
                                    transfer,
                                    result.publications,
                                ) {
                                    first_error.get_or_insert(CrossError::from(error));
                                }
                            }
                        }
                        Err(error) => {
                            first_error.get_or_insert(error);
                        }
                    }
                }
                if let Err(error) = stage.finish(todo.rule) {
                    first_error.get_or_insert(CrossError::from(error));
                }
            }
            if let Some(error) = refused {
                first_error.get_or_insert(error);
            }
            if let Some(error) = first_error {
                let _finished = stage.finish_stage();
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
    ryo_values::domain::rule_requests::errors::RuleRequestsError,
> {
    let failed = |error: &dyn std::fmt::Display| {
        ryo_values::domain::rule_requests::service::failed(&todo.id, error)
    };
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
