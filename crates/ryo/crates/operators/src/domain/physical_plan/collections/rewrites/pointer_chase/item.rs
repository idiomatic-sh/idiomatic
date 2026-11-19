//! Spike (ryo-fmx3): the `pointer_chase` item of the `rewrites` collection. A chain of inner self probes, each reading the
//! one before as both sides with the same key and output and no filter, every link but the last read by nothing else, runs
//! as one `pointer_chase` kernel over the chain's first rows: the links are never made.

use super::super::interface::RewritesItem;
use crate::domain::equality_join::types::{ProbeKey, ProbeMode, ProbeParts};
use crate::domain::physical_plan::collections::kernels::pointer_chase::item::ChaseParts;
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalNode;
use crate::domain::physical_plan::types::{EngineUses, KeyedIndex, NodeReaders, PlanNodeId, RowSource};

/// `pointer_chase`, one item of the `rewrites` collection.
pub(crate) struct PointerChaseItem;

/// The self probe at `at`: what it reads and its probe.
fn self_probe(nodes: &[PhysicalNode], at: usize) -> Option<(PlanNodeId, &ProbeParts)> {
    let Lowering::Probe {
        left: RowSource::Node(left),
        right: RowSource::Node(right),
        probe,
        ..
    } = &nodes.get(at)?.lowering
    else {
        return None;
    };
    (left == right
        && probe.mode == ProbeMode::Inner
        && matches!(probe.key, ProbeKey::Columns { .. })
        && probe.filter.comparisons.is_empty()
        && probe.filter.kept.is_empty()
        && probe.filter.bindings.is_empty())
    .then_some((*left, probe))
}

impl RewritesItem for PointerChaseItem {
    fn rewrite(&self, nodes: &mut [PhysicalNode], _indexes: &mut Vec<KeyedIndex>) {
        if !crate::spike::lazy() || std::env::var_os("RYO_NO_CHASE").is_some() {
            return;
        }
        // The self probe that reads each node, when one does.
        let mut reader: Vec<Option<usize>> = vec![None; nodes.len()];
        for at in 0..nodes.len() {
            if let Some((read, _)) = self_probe(nodes, at)
                && let Some(slot) = reader.get_mut(usize::try_from(read.0).unwrap_or(usize::MAX))
            {
                *slot = Some(at);
            }
        }
        for first in 0..nodes.len() {
            let Some((source, probe)) = self_probe(nodes, first) else {
                continue;
            };
            // A link of a longer chain is taken from that chain's first probe.
            let source_at = usize::try_from(source.0).unwrap_or(usize::MAX);
            if self_probe(nodes, source_at).is_some_and(|(_, before)| before == probe) {
                continue;
            }
            let mut chain = vec![first];
            let mut last = first;
            while let Some(next) = reader.get(last).copied().flatten()
                && nodes.get(last).is_some_and(|node| node.readers == NodeReaders(2) && node.engine == EngineUses(0))
                && self_probe(nodes, next).is_some_and(|(_, step)| step == probe)
            {
                chain.push(next);
                last = next;
            }
            if chain.len() < 2 {
                continue;
            }
            let ProbeKey::Columns { left, right } = &probe.key else {
                continue;
            };
            let chase = ChaseParts {
                left_on: left.clone(),
                right_on: right.clone(),
                output: probe.output.clone(),
                repeated: probe.repeated,
                steps: u32::try_from(chain.len()).unwrap_or(u32::MAX),
            };
            for link in &chain[..chain.len() - 1] {
                if let Some(node) = nodes.get_mut(*link) {
                    node.readers = NodeReaders(0);
                }
            }
            if let Some(node) = nodes.get_mut(source_at) {
                node.readers = NodeReaders(node.readers.0.saturating_sub(1));
            }
            if let Some(node) = nodes.get_mut(last) {
                node.lowering = Lowering::PointerChase {
                    source: RowSource::Node(source),
                    chase,
                };
            }
        }
    }
}
