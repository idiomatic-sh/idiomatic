//! Original column-chain coordinates are lowered once into executable regions.
//! No authored plan, analysis graph, or physical layout survives this boundary.

use crate::domain::{
    column_kernel::types::Chain,
    coverage::types::RuleIndex,
    derived_views::types::{InputsUsed, ViewRequest},
    executable_program::types::{ColumnAlternative, RegionId},
    file_program::types::{ProgramNodeId, RequestGraph, SchemaValidity},
};
use ryo_derived::domain::contract::types::DerivedKind;
use ryo_values::domain::plan_parts::types::{InputNumber, StepId};
use std::sync::Arc;

#[derive(Clone, Debug)]
struct OriginalColumn {
    target: ProgramNodeId,
    filter: ProgramNodeId,
    join: ProgramNodeId,
    left: ProgramNodeId,
    chain: Chain,
}

/// Immutable original-coordinate candidates. The compiler uses these nodes as
/// fusion fences, then drops this value after lowering them into regions.
#[derive(Clone, Debug, Default)]
pub(crate) struct ColumnCandidates {
    candidates: Vec<OriginalColumn>,
}

fn original(graph: &RequestGraph, rule: RuleIndex, step: StepId) -> Option<ProgramNodeId> {
    match graph.program_nodes.get(rule.0)?.get(step.0 as usize)? {
        Ok(node) => Some(*node),
        Err(_) => None,
    }
}

/// Only unobserved, single-reader ancestors may be replaced by one physical instruction.
pub(super) enum ReplacementUse {
    Exclusive,
    Observed,
}
impl ColumnCandidates {
    /// Capture exact original identities before the transient graph is consumed.
    /// Invalid or unmatched authored roots keep their ordinary compiled route.
    pub(crate) fn capture(graph: &RequestGraph) -> Self {
        let mut candidates = Vec::new();
        for (at, node) in graph.nodes.iter().enumerate() {
            for physical in &node.analysis {
                let Some(chain) = &physical.column else {
                    continue;
                };
                let rule = RuleIndex(physical.rule);
                let Some(target) = original(graph, rule, physical.step) else {
                    continue;
                };
                if target != ProgramNodeId(at) {
                    continue;
                }
                let (Some(filter), Some(join), Some(left)) = (
                    original(graph, rule, chain.filter_step),
                    original(graph, rule, chain.join_step),
                    original(graph, rule, chain.left_step),
                ) else {
                    continue;
                };
                if [target, filter, join, left].iter().any(|node| {
                    graph
                        .nodes
                        .get(node.0)
                        .is_none_or(|node| node.schema_valid == SchemaValidity::Invalid)
                }) {
                    continue;
                }
                let Some(join_node) = graph.nodes.get(join.0) else {
                    continue;
                };
                let Some(equality) =
                    join_node.contract.derived.slots[DerivedKind::Equality.ordinal()].as_ref()
                else {
                    continue;
                };
                let primary = &equality.primary;
                if primary.len != InputsUsed(1)
                    || primary.inputs[0] != InputNumber(1)
                    || primary.relations[0] != Some(chain.right)
                    || primary.plan.kind != DerivedKind::Equality
                    || !matches!(
                        &primary.plan.request,
                        ViewRequest::Equality { columns } if columns == &chain.right_on
                    )
                {
                    continue;
                }
                let mut chain = chain.clone();
                chain.index = Some(Arc::clone(&primary.plan));
                candidates.push(OriginalColumn {
                    target,
                    filter,
                    join,
                    left,
                    chain,
                });
            }
        }
        Self { candidates }
    }

    pub(super) fn retain_internal(&mut self, mut usage: impl FnMut(ProgramNodeId) -> ReplacementUse) {
        self.candidates.retain(|candidate| {
            [candidate.join, candidate.filter]
                .into_iter()
                .all(|node| matches!(usage(node), ReplacementUse::Exclusive))
        });
    }
    /// Every original node in a candidate must remain a compiler boundary.
    pub(crate) fn fences(&self) -> impl Iterator<Item = ProgramNodeId> + '_ {
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
                })
            })
            .collect()
    }
}
