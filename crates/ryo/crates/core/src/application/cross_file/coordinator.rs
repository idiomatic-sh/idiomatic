//! Execute existing graph ready batches and transfer publications before their final consumer retires.
use super::errors::CrossError;
use super::{
    service::CrossFile,
    types::{ObservationRun, PublicationSlot},
};
use crate::application::{
    file_dag::{
        observations::cross::{CrossPublications, frontier_complete, frontier_readiness},
        settings::CrossingInput,
    },
    job_execution,
};
use crate::domain::{
    answering::{
        service::settle,
        types::{Answered, Todo},
    },
    coverage::types::RuleIndex,
    file_program::types::ObservationRequest,
    operating::errors::OperatingError,
    reading::service::frontier_bucket,
};
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::{
    plan_parts::types::Input,
    plans::types::StepScope,
    stored_facts::types::ArenaBytes,
};

struct PreparedTask {
    todo: Todo,
    slot: PublicationSlot,
    bindings: Vec<crate::domain::crossing::types::BoundInputWords>,
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
                "observation execution requires the prepared named graph".into(),
            ));
        };
        let graph = run
            .graph
            .observations
            .as_ref()
            .ok_or_else(|| CrossError::Contract("named request graph is absent".into()))?;
        let publication_scope = crate::application::file_dag::scope::ExecutionScope::new(
            std::sync::Arc::clone(&run.graph),
            crate::application::file_dag::scope::ScopePhase::Project,
            true,
            run.executor,
        )?;
        let mut publications =
            CrossPublications::new(graph, std::rc::Rc::clone(publication_scope.owner()), arenas);
        let mut selected = vec![false; run.domain.plans.len()];
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
        if let Ok(readiness) = frontier_readiness(graph, &files, run.domain.observations) {
            // This preparation map drops before execution; logical roots retain the common lease.
            let mut physical = std::collections::BTreeMap::new();
            for (slot, root) in graph.frontiers.iter().enumerate() {
                let needed = graph.outputs[root.producer.0][root.output.0 as usize]
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
                    let words = run.facts.kept(bucket)?;
                    entry.insert(publications.prepare_frontier(words));
                }
                match physical.get(&bucket).ok_or(OperatingError::NotRun)? {
                    Ok(prepared) => publications.install_frontier(slot, prepared, &readiness)?,
                    Err(OperatingError::ObservationIncomplete { .. }) => (),
                    Err(error) => return Err(error.clone().into()),
                }
            }
        }
        for batch in batches {
            let mut tasks = Vec::new();
            for rule in batch {
                let Some(todo) = run.todo.iter().find(|todo| todo.rule == *rule) else {
                    continue;
                };
                let readiness = publications.binding_readiness(*rule)?;
                let plan = run.domain.plans.get(rule.0).ok_or(OperatingError::NotRun)?;
                let dependencies =
                    crate::domain::file_program::service::observations::frontier_dependencies(plan)
                        .map_err(|error| CrossError::Contract(error.to_string()))?;
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
                    executor: run.executor,
                    graph: std::sync::Arc::clone(&run.graph),
                    domain: run.domain.clone(),
                };
                job_execution::service::drain(run.scheduler, run.workers, &tasks, |_, task| {
                    let mut symbols = Symbols::default();
                    let frontier =
                        prepare_frontiers(&run, &task.todo, &task.frontier_ready, &mut symbols)?;
                    self.evaluate_phase(
                        &crossing,
                        &task.todo,
                        frontier,
                        &mut symbols,
                        &task.bindings,
                        task.slot,
                    )
                })?
            };
            for (mut task, result) in tasks.into_iter().zip(results) {
                // Worker handles are gone before the common last-consumer lease retires.
                task.bindings.clear();
                let crossed = result?;
                if let Some(transfer) = settle(
                    &task.todo,
                    crossed,
                    &mut answered.later,
                    &mut answered.fed,
                    &mut answered.unknowns,
                ) {
                    publications.install(task.todo.rule, task.slot, transfer)?;
                }
                publications.finish(task.todo.rule)?;
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
) -> Result<super::types::PreparedFrontiers, crate::domain::rules::errors::RulesError> {
    let failed =
        |error: &dyn std::fmt::Display| crate::domain::rules::service::failed(&todo.id, error);
    let plan = run
        .domain
        .plans
        .get(todo.rule.0)
        .ok_or_else(|| failed(&OperatingError::NotRun))?;
    let first = *run
        .first_bucket
        .get(todo.rule.0)
        .ok_or_else(|| failed(&OperatingError::NotRun))?;
    super::frontiers::prepare(
        plan,
        first,
        run.facts,
        run.project,
        run.project_symbols,
        symbols,
        super::frontiers::PreparationPolicy::Observations { ready },
    )
    .map_err(|error| failed(&error))
}
