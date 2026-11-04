//! Named roots extend the existing file graph's publication and last-consumer lifetime.
use super::{executor::ScopedExecutor, relations::PublishedInput, types::FileDagExecution};
use crate::domain::{
    file_program::types::{
        GraphObservationRoot, NodeId, ObservationSlot, ResolvedObservationProducer,
    },
    operating::{errors::OperatingError, types::OperatorContext},
    table_model::types::Rows,
};
use ryo_relations::domain::relations::types::Relation;
use std::sync::Arc;
pub(crate) mod bound;
pub(crate) mod cross;
mod union;
#[cfg(test)]
mod union_test;

#[derive(Clone)]
enum RootReadiness {
    Pending,
    Evaluated,
    Complete,
    Skipped,
    Unknown(ryo_values::domain::observations::types::ObservationReason),
}
pub(super) enum RootPublication {
    Pending,
    Complete(PublishedInput),
    Unavailable,
}
/// Only named requests allocate these pins; all payloads belong to RelationOwner publications.
pub(super) struct FileObservations {
    roots: Vec<Vec<RootPublication>>,
    readiness: Vec<Vec<RootReadiness>>,
    remaining: Vec<Vec<usize>>,
    bindings: Vec<Vec<Option<PublishedInput>>>,
    completed: Vec<bool>,
}
impl FileObservations {
    pub(super) fn new(graph: &crate::domain::file_program::types::GraphObservations) -> Self {
        Self {
            readiness: graph
                .outputs
                .iter()
                .map(|outputs| vec![RootReadiness::Pending; outputs.len()])
                .collect(),
            roots: graph
                .outputs
                .iter()
                .map(|outputs| {
                    outputs
                        .iter()
                        .map(|output| match output.root {
                            GraphObservationRoot::PerFile(_) => RootPublication::Pending,
                            GraphObservationRoot::CrossFile | GraphObservationRoot::Unavailable => {
                                RootPublication::Unavailable
                            }
                        })
                        .collect()
                })
                .collect(),
            remaining: graph
                .outputs
                .iter()
                .map(|outputs| {
                    outputs
                        .iter()
                        .map(|output| output.consumers.len())
                        .collect()
                })
                .collect(),
            bindings: graph
                .bindings
                .iter()
                .map(|inputs| inputs.iter().map(|_| None).collect())
                .collect(),
            completed: vec![false; graph.bindings.len()],
        }
    }
}
impl FileDagExecution {
    /// This boundary runs under the producer's own context, after its actual root evaluation.
    pub(super) fn publish_observed_node(&mut self, node: NodeId, table: &PublishedInput) {
        let complete = matches!(
            self.coverage.get(node.0),
            Some(Some(super::completion::CoverageState::Complete))
        ) && self
            .graph
            .nodes
            .get(node.0)
            .is_some_and(|node| self.completion.observation_ready(&node.publication_reads));
        let (Some(graph), Some(state)) = (&self.graph.observations, &mut self.observations) else {
            return;
        };
        let Some(producer) = self.current_rule else {
            return;
        };
        if let Some(outputs) = graph
            .publish
            .get(producer)
            .and_then(|nodes| nodes.get(&node))
        {
            let roots = state
                .roots
                .get_mut(producer)
                .expect("prepared producer has publication slots");
            for output in outputs {
                let slot = roots
                    .get_mut(output.0 as usize)
                    .expect("prepared publication output index");
                if complete {
                    *slot = RootPublication::Complete(table.clone());
                    let readiness = &mut state.readiness[producer][output.0 as usize];
                    if !matches!(readiness, RootReadiness::Complete) {
                        *readiness = RootReadiness::Evaluated;
                    }
                } else {
                    *slot = RootPublication::Unavailable;
                    state.readiness[producer][output.0 as usize] = RootReadiness::Unknown(
                        ryo_values::domain::observations::types::ObservationReason(
                            "producer physical inputs are incomplete".into(),
                        ),
                    );
                }
            }
        }
    }
    pub(super) fn observation_slot(
        &self,
        id: crate::domain::file_program::types::ObservationGraphInputId,
    ) -> Result<ObservationSlot, OperatingError> {
        self.graph
            .observations
            .as_ref()
            .and_then(|graph| graph.slots.get(id.0))
            .copied()
            .ok_or_else(unavailable)
    }
    /// Missing publication is Unknown: this method never invokes a producer operation.
    pub(super) fn observed_scoped<'f>(
        execution: &dyn super::service::ScopeState,
        slot: ObservationSlot,
        read: &dyn Fn(Relation) -> Result<Rows<'f>, OperatingError>,
        executor: &ScopedExecutor<'_>,
        context: &mut OperatorContext<'_>,
    ) -> Result<PublishedInput, OperatingError> {
        let (graph, position, sources, kinds, contract) = {
            let scope = execution.state();
            let graph = Arc::clone(&scope.graph);
            let named = graph.observations.as_ref().ok_or_else(unavailable)?;
            if let Some(at) = named.slots.iter().position(|each| *each == slot)
                && let Some(provided) = scope
                    .provided_observations
                    .get(&crate::domain::file_program::types::ObservationGraphInputId(at))
            {
                return provided.clone();
            }
            let (position, binding) = named
                .bindings
                .get(slot.rule)
                .and_then(|inputs| {
                    inputs
                        .iter()
                        .enumerate()
                        .find(|(_, input)| input.input.input == slot.input)
                })
                .ok_or_else(unavailable)?;
            let state = scope.observations.as_ref().ok_or_else(unavailable)?;
            if let Some(Some(table)) = state
                .bindings
                .get(slot.rule)
                .and_then(|inputs| inputs.get(position))
            {
                return Ok(table.clone());
            }
            let mut sources = Vec::with_capacity(binding.input.producers.len());
            for producer in &binding.input.producers {
                let ResolvedObservationProducer::Output { rule, output } = producer else {
                    return Err(unavailable());
                };
                match state
                    .roots
                    .get(rule.0)
                    .and_then(|outputs| outputs.get(output.0 as usize))
                {
                    Some(RootPublication::Complete(table)) => sources.push(table.clone()),
                    Some(RootPublication::Pending | RootPublication::Unavailable) | None => {
                        return Err(unavailable());
                    }
                }
            }
            if !std::rc::Rc::ptr_eq(&scope.relations, executor.owner()) {
                return Err(OperatingError::NotRead);
            }
            let kinds = binding
                .input
                .schema
                .columns
                .iter()
                .map(|column| column.kind)
                .collect::<Vec<_>>();
            let contract = Arc::clone(binding.union.as_ref().map_err(Clone::clone)?);
            (graph, position, sources, kinds, contract)
        };
        let result = union::combine(&sources, &kinds, &contract, read, executor, context)?;
        let mut scope = execution.state_mut();
        let state = scope.observations.as_mut().ok_or_else(unavailable)?;
        *state
            .bindings
            .get_mut(slot.rule)
            .and_then(|inputs| inputs.get_mut(position))
            .ok_or_else(unavailable)? = Some(result.clone());
        drop(graph);
        Ok(result)
    }
    /// Applicability is not a complete empty producer.
    pub(crate) fn skip_observation_rule(&mut self, rule: usize) {
        if let Some(state) = &mut self.observations {
            if let Some(readiness) = state.readiness.get_mut(rule) {
                readiness.fill(RootReadiness::Skipped);
            }
        }
        self.finish_observation_rule(rule);
    }
    /// A completed node is not durable evidence until its actual words have
    /// been accepted by FactsWorker::keep (including a real zero-row bucket).
    pub(crate) fn commit_frontier(
        &mut self,
        rule: usize,
        frontier: ryo_values::domain::plan_parts::types::InputNumber,
    ) {
        let (Some(graph), Some(state)) = (&self.graph.observations, &mut self.observations) else {
            return;
        };
        for root in graph
            .frontiers
            .iter()
            .filter(|root| root.producer.0 == rule && root.frontier == frontier)
        {
            if let Some(slot) = state
                .readiness
                .get_mut(rule)
                .and_then(|roots| roots.get_mut(root.output.0 as usize))
                && matches!(slot, RootReadiness::Evaluated)
            {
                *slot = RootReadiness::Complete;
            }
        }
    }
    /// Snapshot only the precomputed demanded frontier slots. Retired handles do not erase
    /// evidence that the producer actually completed before its words were retained.
    pub(crate) fn observation_snapshot(
        &self,
    ) -> Option<std::sync::Arc<ryo_values::domain::records::types::FrontierReadiness>> {
        use ryo_values::domain::observations::{
            service::{new_frontiers, set_frontier},
            types::{ObservationReadinessCount, ObservationReadinessSlot, ProducerReadiness},
        };
        let (Some(graph), Some(state)) = (&self.graph.observations, &self.observations) else {
            return None;
        };
        if graph.frontiers.is_empty() {
            return None;
        }
        let mut map = new_frontiers(
            graph.layout,
            ObservationReadinessCount(
                u32::try_from(graph.frontiers.len())
                    .expect("prepared frontier status count fits u32"),
            ),
        );
        for (slot, root) in graph.frontiers.iter().enumerate() {
            let readiness = match &state.readiness[root.producer.0][root.output.0 as usize] {
                RootReadiness::Pending | RootReadiness::Evaluated => continue, // unset is Unknown; no per-root reason allocation
                RootReadiness::Complete => ProducerReadiness::Complete,
                RootReadiness::Skipped => ProducerReadiness::Skipped,
                RootReadiness::Unknown(reason) => ProducerReadiness::Unknown(reason.clone()),
            };
            set_frontier(
                &mut map,
                ObservationReadinessSlot(
                    u32::try_from(slot).expect("prepared frontier slot fits u32"),
                ),
                readiness,
            )
            .expect("fresh prepared status slot is valid");
        }
        Some(std::sync::Arc::new(map))
    }
    pub(super) fn refuse_observation(
        &mut self,
        rule: usize,
        output: ryo_values::domain::observations::types::ObservationIndex,
        reason: ryo_values::domain::observations::types::ObservationReason,
    ) -> Result<(), OperatingError> {
        let state = self.observations.as_mut().ok_or_else(unavailable)?;
        *state
            .readiness
            .get_mut(rule)
            .and_then(|outputs| outputs.get_mut(output.0 as usize))
            .ok_or(OperatingError::NotRun)? = RootReadiness::Unknown(reason);
        Ok(())
    }
    /// Every admitted or skipped consumer closes its original-index edge once.
    pub(crate) fn finish_observation_rule(&mut self, rule: usize) {
        let (Some(graph), Some(state)) = (&self.graph.observations, &mut self.observations) else {
            return;
        };
        let Some(completed) = state.completed.get_mut(rule) else {
            return;
        };
        if *completed {
            return;
        }
        *completed = true;
        if let Some(inputs) = state.bindings.get_mut(rule) {
            inputs.iter_mut().for_each(|input| *input = None);
        }
        if let Some(roots) = state.roots.get_mut(rule) {
            for root in roots {
                if matches!(root, RootPublication::Pending) {
                    *root = RootPublication::Unavailable;
                }
            }
        }
        for edge in graph
            .retire
            .get(rule)
            .expect("prepared named consumer has retirement edges")
        {
            let remaining = state
                .remaining
                .get_mut(edge.producer.0)
                .and_then(|outputs| outputs.get_mut(edge.output.0 as usize))
                .expect("retirement edge names a prepared producer output");
            *remaining = remaining
                .checked_sub(1)
                .expect("each prepared retirement edge closes exactly once");
            if *remaining == 0 {
                state.roots[edge.producer.0][edge.output.0 as usize] = RootPublication::Unavailable;
            }
        }
    }
}
fn unavailable() -> OperatingError {
    OperatingError::ObservationIncomplete {
        reason: "a bound producer has no complete publication in this generation".into(),
    }
}
