//! The `tree_probe` item of the `rewrites` collection: a probe of a tree relation keyed by one node column reads the open
//! file's tree as its index, each left row walking from its own node, when the tree answers exactly what an index of the
//! read would: not ordered, and every comparison of its right filter one that cannot fail.

use super::super::interface::RewritesItem;
use crate::domain::equality_join::types::{ProbeKey, ProbeMode};
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalNode;
use crate::domain::physical_plan::types::{KeyedIndex, ProbeIndex, RowSource};
use ryo_values::domain::plan_parts::service::rides_the_walk;

/// `tree_probe`, one item of the `rewrites` collection.
pub(crate) struct TreeProbeItem;

impl RewritesItem for TreeProbeItem {
    fn rewrite(&self, nodes: &mut [PhysicalNode], _indexes: &mut Vec<KeyedIndex>) {
        for node in nodes {
            if let Lowering::Probe {
                right: RowSource::Tree(relation),
                probe,
                index,
                ..
            } = &mut node.lowering
                && let ProbeKey::Columns { left, right } = &probe.key
                && let ([_], [on]) = (left.as_slice(), right.as_slice())
                && !matches!(probe.mode, ProbeMode::Ordered(_))
                && rides_the_walk(*relation, &[*on], &[], &probe.filter.comparisons).is_some()
            {
                *index = ProbeIndex::Tree;
            }
        }
    }
}
