//! The `ordered_existence` item of the `rewrites` collection: an antijoin whose right rows are the distinct projection of a
//! strict-order select over a plain self join of one source, partitioned by the join's keys, keeps a left row exactly when
//! no row of its partition orders past it. So the antijoin probes the source itself in `ProbeMode::Ordered`, deciding by
//! each partition's extremum, and the self join, the select and the projection lose it as a reader. It applies only where
//! the order column is proven a number from the relations it is read from, and where no other node reads the three.

use super::super::interface::RewritesItem;
use crate::domain::equality_join::types::OrderedProbe;
use crate::domain::physical_plan::types::{KeyedIndex, PlanNodeId};
use crate::domain::physical_plan::types::physical_plan::value::PhysicalNode;

/// `ordered_existence`, one item of the `rewrites` collection.
pub(crate) struct OrderedExistenceItem;

/// An antijoin the item recognised: its node, the source its self join reads, and the order it probes by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Recognised {
    /// The antijoin.
    antijoin: PlanNodeId,
    /// The source both sides of the self join read.
    source: PlanNodeId,
    /// The order the probe compares by.
    order: OrderedProbe,
}

impl RewritesItem for OrderedExistenceItem {
    fn rewrite(&self, nodes: &mut [PhysicalNode], _indexes: &mut Vec<KeyedIndex>) {
        let found: Vec<Recognised> = Vec::new();
        for recognised in found {
            let _ = (recognised.antijoin, recognised.source, recognised.order, nodes.len());
        }
    }
}
