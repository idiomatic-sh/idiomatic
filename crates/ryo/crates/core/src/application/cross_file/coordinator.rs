//! Execute declared ready batches and transfer publications before their final consumer retires.
use super::errors::CrossError;
use super::{
    service::CrossFile,
    types::{ObservationRun, PublicationSlot},
};
use crate::application::file_dag::{
    observations::cross::{CrossPublications, frontier_complete, frontier_readiness},
    settings::CrossingInput,
};
use {crate::domain::answering::service::settle, crate::domain::answering::types::Answered, crate::domain::answering::types::Todo, crate::domain::coverage::types::RuleIndex, crate::domain::file_program::types::ObservationRequest, crate::domain::operating::errors::OperatingError, ryo_values::domain::stored_facts::service::frontier_bucket};
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::{
    plan_parts::types::Input, plans::types::StepScope, stored_facts::types::ArenaBytes,
};

struct PreparedTask {
    todo: Todo,
    slot: PublicationSlot,
    bindings: Vec<super::types::BoundInput>,
    frontier_ready: Vec<bool>,
}
impl CrossFile {
    pub(crate) fn observations(
        &self,
        run: ObservationRun<'_>,
        arenas: &mut Vec<ArenaBytes>,
        answered: &mut Answered,
    ) -> Result<(), CrossError> {
        let ObservationRequest::Bound { batches, .. } = run.observations else {
            return Err(CrossError::Contract(
                "observation execution requires the prepared named contract".into(),
            ));
        };
        let metadata = run
            .program
            .observation_metadata()
            .ok_or_else(|| CrossError::Contract("named request metadata is absent".into()))?;
        let owner = crate::application::file_dag::program::facade::publication_owner(
            &run.program,
            run.services,
        )?;
        let mut publications = CrossPublications::new(metadata, &owner, arenas);
        let mut selected = vec![false; run.domain.metadata.len()];
        for todo in run.todo {
            *selected
                .get_mut(todo.rule.0)
                .ok_or(OperatingError::NotRun)? = true;
        }
        for (rule, selected) in selected.iter().enumerate() {
            if !selected {
                publications.finish(RuleIndex(rule))?;
            }
        }
        let files = (0..run.domain.files.len())
            .map(|file| {
                u32::try_from(file)
                    .map(ryo_relations::domain::relations::types::FileId)
                    .map_err(|_| OperatingError::NotRun)
            })
            .collect::<Result<Vec<_>, _>>()?;
        // Missing/old/refused readiness leaves roots unavailable. It never changes
        // an independent Emit or fabricates an empty producer population.
        if let Ok(readiness) = frontier_readiness(metadata, &files, run.domain.observations) {
            // This preparation map drops before execution; logical roots retain the common lease.
            let mut physical = std::collections::BTreeMap::new();
            for (slot, root) in metadata.frontiers.iter().enumerate() {
                let needed = metadata.outputs[root.producer.0][root.output.0 as usize]
                    .consumers
                    .iter()
                    .any(|consumer| selected[*consumer]);
                if !needed || !frontier_complete(slot, &readiness)? {
                    continue;
                }
                let first = *run
                    .first_bucket
                    .get(root.producer.0)
                    .ok_or(OperatingError::NotRun)?;
                let bucket = frontier_bucket(first, root.frontier);
                if let std::collections::btree_map::Entry::Vacant(entry) = physical.entry(bucket) {
                    let payload = run.facts.kept(bucket)?;
                    let required =
                        crate::application::file_dag::program::facade::required_partitions(
                            &run.program,
                            root.producer,
                            run.domain.files,
                            run.domain.covered,
                            run.domain.skipped,
                            run.services,
                        )?;
                    let verified = super::frontiers::validate_bucket(
                        &run.program,
                        root.producer,
                        root.frontier.0,
                        first,
                        bucket,
                        &required,
                        &payload,
                    );
                    let prepared = match verified {
                        Ok(crate::application::file_dag::completion::CoverageState::Complete) => {
                            crate::application::file_dag::program::facade::validate_restored_frontier(
                                &run.program,
                                root.producer,
                                root.frontier,
                                first,
                                bucket,
                                &required,
                                payload,
                                &owner,
                            )
                            .and_then(|token| publications.prepare_frontier(token))
                        }
                        Ok(crate::application::file_dag::completion::CoverageState::Partial {
                            ..
                        }) => Err(OperatingError::ObservationIncomplete {
                            reason: "named producer frontier has incomplete physical coverage"
                                .into(),
                        }),
                        Err(error) => Err(error),
                    };
                    entry.insert(prepared);
                }
                match physical.get(&bucket).ok_or(OperatingError::NotRun)? {
                    Ok(prepared) => publications.install_frontier(slot, prepared, &readiness)?,
                    Err(OperatingError::ObservationIncomplete { .. }) => (),
                    Err(error) => return Err(error.clone().into()),
                }
            }
        }
        // The retained project execution and every decoded text place share one
        // live symbol generation across all producer and consumer batches.
        let mut symbols = Symbols::default();
        for batch in batches {
            let mut tasks = Vec::new();
            for rule in batch {
                let Some(todo) = run.todo.iter().find(|todo| todo.rule == *rule) else {
                    continue;
                };
                let readiness = publications.binding_readiness(*rule)?;
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
                    slot: publications.reserve(*rule)?,
                    bindings: publications.bindings(*rule)?,
                    frontier_ready,
                });
            }
            if tasks.is_empty() {
                continue;
            }
            let results = {
                let crossing = CrossingInput {
                    arenas: publications.arenas(),
                    services: run.services,
                    program: std::sync::Arc::clone(&run.program),
                    domain: run.domain.clone(),
                };
                tasks
                    .iter()
                    .map(|task| {
                        let frontier = prepare_frontiers(
                            &run,
                            &task.todo,
                            &task.frontier_ready,
                            &mut symbols,
                        )?;
                        self.evaluate_phase(
                            &crossing,
                            &task.todo,
                            frontier,
                            &mut symbols,
                            &task.bindings,
                            task.slot,
                            &owner,
                        )
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
                            if let Err(error) = publications.install(
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
                if let Err(error) = publications.finish(task.todo.rule) {
                    first_error.get_or_insert(CrossError::from(error));
                }
            }
            if let Some(error) = first_error {
                return Err(error);
            }
        }
        Ok(())
    }
}
fn prepare_frontiers(
    run: &ObservationRun<'_>,
    todo: &Todo,
    ready: &[bool],
    symbols: &mut Symbols,
) -> Result<super::types::PreparedFrontiers, ryo_values::domain::rule_requests::errors::RulesError> {
    let failed =
        |error: &dyn std::fmt::Display| crate::domain::rules::service::failed(&todo.id, error);
    let first = *run
        .first_bucket
        .get(todo.rule.0)
        .ok_or_else(|| failed(&OperatingError::NotRun))?;
    let required = crate::application::file_dag::program::facade::required_partitions(
        &run.program,
        todo.rule,
        run.domain.files,
        run.domain.covered,
        run.domain.skipped,
        run.services,
    )
    .map_err(|error| failed(&error))?;
    super::frontiers::prepare(
        run.program.as_ref(),
        todo.rule,
        first,
        run.facts,
        run.project,
        run.project_symbols,
        symbols,
        &required,
        super::frontiers::PreparationPolicy::Observations { ready },
    )
    .map_err(|error| failed(&error))
}
