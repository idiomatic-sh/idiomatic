//! The `scan_node` item of the `kernels` collection: an earlier node's materialized rows, taken out of the buffers while its
//! reader reads them and given back after, with the fault it raised after its rows.

#[cfg(test)]
use super::super::interface::{KernelsItem, Reach};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::{
    Lowering, NodeReaders, PlanNodeId, Retention, RowSource, TakenRows,
};
use ryo_values::domain::rows::types::Table;

/// `scan_node`, one item of the `kernels` collection.
#[cfg(test)]
pub(crate) struct ScanNodeItem;

#[cfg(test)]
impl KernelsItem for ScanNodeItem {
    fn reached_by(&self, lowering: &Lowering) -> Reach {
        let runs = { sources_of(lowering).any(|source| matches!(source, RowSource::Node(_))) };
        if runs { Reach::Runs } else { Reach::Skips }
    }
}

/// Takes `read`'s rows out of the buffers for its reader.
///
/// # Errors
///
/// `NotRun` when the node holds no rows.
pub(crate) fn take(
    buffers: &mut FileBuffers,
    read: PlanNodeId,
) -> Result<TakenRows, PortableOperatorFault> {
    let place = usize::try_from(read.0).unwrap_or(usize::MAX);
    let rows = buffers
        .nodes
        .get_mut(place)
        .and_then(Option::take)
        .ok_or(PortableOperatorFault::NotRun)?;
    let pending = buffers.pending.get_mut(place).and_then(Option::take);
    Ok(TakenRows { rows, pending })
}

/// Gives a read input back after `readings` of its readers are done with it; it is released instead when they were its last readers
/// and no consumer keeps it.
pub(crate) fn give_back(
    plan: &PhysicalPlan,
    buffers: &mut FileBuffers,
    read: PlanNodeId,
    taken: TakenRows,
    readings: NodeReaders,
) {
    let place = usize::try_from(read.0).unwrap_or(usize::MAX);
    let left = buffers.remaining.get_mut(place).map_or(0, |left| {
        left.0 = left.0.saturating_sub(readings.0);
        left.0
    });
    let retained = plan
        .nodes
        .get(place)
        .is_some_and(|node| node.retained == Retention::Retained);
    if left > 0 || retained {
        if let Some(held) = buffers.nodes.get_mut(place) {
            *held = Some(taken.rows);
        }
        if let Some(pending) = buffers.pending.get_mut(place) {
            *pending = taken.pending;
        }
    }
}

/// Holds a node's rows and pending fault, with all its readers still to come.
pub(crate) fn hold(
    plan: &PhysicalPlan,
    buffers: &mut FileBuffers,
    node: PlanNodeId,
    rows: Table,
    pending: Option<PortableOperatorFault>,
) {
    let place = usize::try_from(node.0).unwrap_or(usize::MAX);
    if let Some(held) = buffers.nodes.get_mut(place) {
        *held = Some(rows);
    }
    if let Some(slot) = buffers.pending.get_mut(place) {
        *slot = pending;
    }
    if let (Some(left), Some(physical)) = (buffers.remaining.get_mut(place), plan.nodes.get(place))
    {
        *left = physical.readers;
    }
}

/// The rows a lowered node reads, in the order its kernels read them; a lookup reads its index's relation.
pub(crate) fn sources_of(lowering: &Lowering) -> impl Iterator<Item = RowSource> {
    let (first, second) = match lowering {
        Lowering::Filter { source, .. }
        | Lowering::Project { source, .. }
        | Lowering::Extend { source, .. }
        | Lowering::Group { source, .. }
        | Lowering::TextMap { source, .. }
        | Lowering::Render { source, .. } => (Some(*source), None),
        Lowering::Probe { left, right, .. } | Lowering::OrderedMatch { left, right, .. } => {
            (Some(*left), Some(*right))
        }
        Lowering::Distinct {
            sources: [left, right],
        } => (Some(*left), Some(*right)),
        Lowering::Lookup { .. } | Lowering::Unlowered => (None, None),
    };
    first.into_iter().chain(second)
}
