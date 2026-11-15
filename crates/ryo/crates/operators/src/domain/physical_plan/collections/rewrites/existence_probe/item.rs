//! The `existence_probe` item of the `rewrites` collection: a semi or anti probe stops at the first right row of a key, which
//! decides as reading all of them does.

use super::super::interface::RewritesItem;
use crate::domain::equality_join::types::{Existence, ProbeMode};
use crate::domain::physical_plan::types::KeyedIndex;
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalNode;

/// `existence_probe`, one item of the `rewrites` collection.
pub(crate) struct ExistenceProbeItem;

impl RewritesItem for ExistenceProbeItem {
    fn rewrite(&self, nodes: &mut [PhysicalNode], _indexes: &mut Vec<KeyedIndex>) {
        for node in nodes {
            if let Lowering::Probe { probe, .. } = &mut node.lowering
                && matches!(probe.mode, ProbeMode::Semi | ProbeMode::Anti)
            {
                probe.existence = Existence::First;
            }
        }
    }
}
