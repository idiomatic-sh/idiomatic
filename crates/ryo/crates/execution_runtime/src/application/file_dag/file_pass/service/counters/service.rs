//! The node counters of a run, emitted once when the file pass's program is dropped at the run's end: every worker's counts
//! summed, one event per node made, with its kind and the rules whose steps it is, then one event for the run: how many node
//! makings there were, how many wrote no row, and the time of each.
use crate::application::file_dag::file_pass::constants::{
    NODE_COUNTERS_NODE, NODE_COUNTERS_RUN, NODE_COUNTERS_TARGET,
};
use crate::application::file_dag::file_pass::types::PassProgram;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::executable_program::service::{physical, rule};
use ryo_operators::domain::physical_plan::types::node_counters::value::{
    NodeCounters, NodeKind, NodeWork,
};
use ryo_values::domain::runs::types::RuleId;
use std::sync::PoisonError;

impl Drop for PassProgram {
    fn drop(&mut self) {
        let scratch = self
            .scratch
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner);
        let mut summed: Option<NodeCounters> = None;
        for each in scratch.iter_mut() {
            if let Some(counters) = each.buffers.take_counters() {
                summed.get_or_insert_default().merge(counters);
            }
        }
        if let Some(summed) = summed {
            emitted(self, &summed);
        }
    }
}

/// Emits `counters`, the counts of `pass`'s nodes over the run: one `node` event per node made, then one `run` event.
fn emitted(pass: &PassProgram, counters: &NodeCounters) {
    let rules = rules_of(pass);
    let plan = physical(&pass.program);
    let mut run = NodeWork::default();
    let mut nodes = 0_usize;
    for counted in counters.counts() {
        let work = counted.work;
        // A node made is one of the plan's: the plan the run made it of.
        let kind = plan.node_kind(counted.node).unwrap_or(NodeKind::Unlowered);
        let named: &[&RuleId] = rules
            .get(usize::try_from(counted.node.0).unwrap_or(usize::MAX))
            .map_or(&[], Vec::as_slice);
        tracing::event!(
            name: NODE_COUNTERS_NODE,
            target: NODE_COUNTERS_TARGET,
            tracing::Level::INFO,
            node = counted.node.0,
            kind = ?kind,
            runs = work.runs.0,
            empty_runs = work.empty_runs.0,
            rows_in = work.rows_in.0,
            rows_out = work.rows_out.0,
            nanos = work.time.as_nanos(),
            empty_nanos = work.empty_time.as_nanos(),
            rules = ?named,
        );
        run.add(&work);
        nodes = nodes.saturating_add(1);
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "a share for a reader: the counts are far below the 2^53 makings past which it would round"
    )]
    let empty_share = if run.runs.0 == 0 {
        0.0
    } else {
        run.empty_runs.0 as f64 / run.runs.0 as f64
    };
    tracing::event!(
        name: NODE_COUNTERS_RUN,
        target: NODE_COUNTERS_TARGET,
        tracing::Level::INFO,
        nodes,
        runs = run.runs.0,
        empty_runs = run.empty_runs.0,
        empty_share,
        rows_in = run.rows_in.0,
        rows_out = run.rows_out.0,
        nanos = run.time.as_nanos(),
        empty_nanos = run.empty_time.as_nanos(),
    );
}

/// The ids of the rules whose steps each node is, by node, in rule order. The pass holds an id for each rule it plans: both
/// come from the same request.
fn rules_of(pass: &PassProgram) -> Vec<Vec<&RuleId>> {
    let mut rules: Vec<Vec<&RuleId>> = vec![Vec::new(); physical(&pass.program).node_count().0];
    for (at, id) in pass.ids.iter().enumerate() {
        let Ok(nodes) = rule(&pass.program, RuleIndex(at)) else {
            continue;
        };
        let steps = nodes.steps.iter().chain(std::iter::once(&nodes.emit));
        for node in steps.flatten() {
            if let Some(named) = rules.get_mut(usize::try_from(node.0).unwrap_or(usize::MAX))
                && !named.contains(&id)
            {
                named.push(id);
            }
        }
    }
    rules
}
