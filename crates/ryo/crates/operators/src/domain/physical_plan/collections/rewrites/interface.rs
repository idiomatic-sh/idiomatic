//! The shape every item of the `rewrites` collection implements.

use crate::domain::physical_plan::types::{KeyedIndex};
use crate::domain::physical_plan::types::physical_plan::value::PhysicalNode;

/// What every item of the `rewrites` collection offers: an equivalence over a plan's lowered nodes.
pub(crate) trait RewritesItem {
    /// Rewrites the nodes in place, adding each index it shares to `indexes`; the plan answers the same rows after.
    fn rewrite(&self, nodes: &mut [PhysicalNode], indexes: &mut Vec<KeyedIndex>);
}
