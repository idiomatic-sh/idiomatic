//! Original column-chain coordinates are lowered once into executable regions.
//! No authored plan, analysis graph, or physical layout survives this boundary.

use crate::domain::{
    column_kernel::types::Chain,
    coverage::types::RuleIndex,
    executable_program::types::{HeldJoinOrder, RegionId},
    file_program::types::{NodeId, RequestGraph},
};
use ryo_derived::domain::contract::types::DerivedKind;
use ryo_values::domain::{
    plan_parts::types::{Input, StepId},
    plans::types::Operation,
};
use std::sync::Arc;

#[derive(Clone, Debug)]
struct OriginalColumn {
    target: NodeId,
    filter: NodeId,
    join: NodeId,
    left: NodeId,
    chain: Chain,
    order: HeldJoinOrder,
}

/// Immutable original-coordinate candidates. The compiler uses these nodes as
/// fusion fences, then drops this value after lowering them into regions.
#[derive(Clone, Debug, Default)]
pub(crate) struct ColumnCandidates {
    candidates: Vec<OriginalColumn>,
}

/// The typed semantic shape required by the shared physical invocation.
#[derive(Clone, Debug)]
pub(crate) struct ColumnAlternative {
    target: RegionId,
    left: RegionId,
    join: RegionId,
    filter: RegionId,
    chain: Chain,
    order: HeldJoinOrder,
    cancelled: [NodeId; 2],
}

impl ColumnAlternative {
    pub(crate) fn target(&self) -> RegionId {
        self.target
    }

    pub(crate) fn left(&self) -> RegionId {
        self.left
    }

    pub(crate) fn join(&self) -> RegionId {
        self.join
    }

    pub(crate) fn filter(&self) -> RegionId {
        self.filter
    }

    pub(crate) fn chain(&self) -> &Chain {
        &self.chain
    }

    pub(crate) fn order(&self) -> HeldJoinOrder {
        self.order
    }

    /// Original Join and equality-filter nodes replaced by the physical chain.
    pub(crate) fn cancelled(&self) -> [NodeId; 2] {
        self.cancelled
    }
}

fn original(graph: &RequestGraph, rule: RuleIndex, step: StepId) -> Option<NodeId> {
    match graph.program_nodes.get(rule.0)?.get(step.0 as usize)? {
        Ok(node) => Some(*node),
        Err(_) => None,
    }
}

impl ColumnCandidates {
    /// Capture exact original identities before the transient graph is consumed.
    /// Invalid or unmatched authored roots keep their ordinary compiled route.
    pub(crate) fn capture(graph: &RequestGraph) -> Self {
        let mut candidates = Vec::new();
        for (at, node) in graph.nodes.iter().enumerate() {
            for physical in &node.regions {
                let Some(chain) = &physical.column else {
                    continue;
                };
                let rule = RuleIndex(physical.rule);
                let Some(target) = original(graph, rule, physical.step) else {
                    continue;
                };
                if target != NodeId(at) {
                    continue;
                }
                let plan = physical.plan.as_ref();
                let Some(Operation::Project {
                    from: Input::Step(filter_step),
                    ..
                }) = plan
                    .steps
                    .get(physical.step.0 as usize)
                    .map(|step| &step.operation)
                else {
                    continue;
                };
                let Some(Operation::Select {
                    from: Input::Step(join_step),
                    ..
                }) = plan
                    .steps
                    .get(filter_step.0 as usize)
                    .map(|step| &step.operation)
                else {
                    continue;
                };
                let Some(Operation::Join {
                    left: Input::Step(left_step),
                    ..
                }) = plan
                    .steps
                    .get(join_step.0 as usize)
                    .map(|step| &step.operation)
                else {
                    continue;
                };
                if left_step.0 as usize != chain.left_step {
                    continue;
                }
                let (Some(filter), Some(join), Some(left)) = (
                    original(graph, rule, *filter_step),
                    original(graph, rule, *join_step),
                    original(graph, rule, *left_step),
                ) else {
                    continue;
                };
                if [target, filter, join, left].iter().any(|node| {
                    graph
                        .nodes
                        .get(node.0)
                        .is_none_or(|node| !node.schema_valid)
                }) {
                    continue;
                }
                let Some(join_node) = graph.nodes.get(join.0) else {
                    continue;
                };
                let Some(equality) =
                    join_node.contract.derived.collected[DerivedKind::Equality.ordinal()].as_ref()
                else {
                    continue;
                };
                let mut chain = chain.clone();
                chain.index = Some(Arc::clone(&equality.primary.plan));
                let order = if join_node.native_join_right_first {
                    HeldJoinOrder::RightBeforeLeft
                } else {
                    HeldJoinOrder::LeftWithEmptyGate
                };
                candidates.push(OriginalColumn {
                    target,
                    filter,
                    join,
                    left,
                    chain,
                    order,
                });
            }
        }
        Self { candidates }
    }

    /// Every original node in a candidate must remain a compiler boundary.
    pub(crate) fn fences(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.candidates
            .iter()
            .flat_map(|each| [each.target, each.filter, each.join, each.left])
    }

    /// Replace original node coordinates with final executable region identities.
    /// A missing mapping declines only this physical alternative before work.
    pub(crate) fn lower(&self, regions: &[RegionId]) -> Vec<ColumnAlternative> {
        self.candidates
            .iter()
            .filter_map(|each| {
                let (Some(&target), Some(&left), Some(&join), Some(&filter)) = (
                    regions.get(each.target.0),
                    regions.get(each.left.0),
                    regions.get(each.join.0),
                    regions.get(each.filter.0),
                ) else {
                    return None;
                };
                Some(ColumnAlternative {
                    target,
                    left,
                    join,
                    filter,
                    chain: each.chain.clone(),
                    order: each.order,
                    cancelled: [each.join, each.filter],
                })
            })
            .collect()
    }
}
